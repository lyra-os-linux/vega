//! Conservative import gate, not a disk image parser. QEMU remains responsible
//! for validating the image. Reject references to other host files before QEMU
//! ever opens an imported image. Format is explicit, never auto-detected.
use super::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Iso,
    Qcow2,
    Raw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Firmware {
    Bios,
    Uefi,
}

impl MediaKind {
    pub fn disk_format(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            _ => "qcow2",
        }
    }

    pub(crate) fn check_prefix(self, bytes: &[u8]) -> Result<()> {
        if self != Self::Qcow2 {
            return Ok(());
        }
        // https://www.qemu.org/docs/master/interop/qcow2.html (big-endian).
        if bytes.len() < 104 || &bytes[..4] != b"QFI\xfb" {
            return Err("Select a valid QCOW2 disk, or choose the RAW format explicitly.".into());
        }
        let u32_at = |offset| u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let u64_at = |offset| u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        let version = u32_at(4);
        if !matches!(version, 2 | 3)
            || !(9..=21).contains(&u32_at(20))
            || u64_at(24) == 0
            || u64_at(24) > (2048_u64 << 30)
        {
            return Err("Unsupported QCOW2 header or virtual disk size.".into());
        }
        if u64_at(8) != 0 || u32_at(16) != 0 || u32_at(32) != 0 || u32_at(60) != 0 {
            return Err("Import requires a standalone, unencrypted QCOW2 disk without backing files or internal snapshots.".into());
        }
        // Includes dirty/corrupt/external-data and future unknown features.
        // This deliberately rejects advanced valid images instead of guessing.
        if version == 3
            && (u64_at(72) != 0
                || u64_at(88) != 0
                || u32_at(100) < 104
                || u32_at(100) % 8 != 0
                || u32_at(100) > (1 << u32_at(20)))
        {
            return Err(
                "QCOW2 features require conversion to a clean standalone image before import."
                    .into(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn header() -> Vec<u8> {
        let mut h = vec![0; 104];
        h[..4].copy_from_slice(b"QFI\xfb");
        h[4..8].copy_from_slice(&3_u32.to_be_bytes());
        h[20..24].copy_from_slice(&16_u32.to_be_bytes());
        h[24..32].copy_from_slice(&(4_u64 << 30).to_be_bytes());
        h[100..104].copy_from_slice(&104_u32.to_be_bytes());
        h
    }
    #[test]
    fn rejects_external_references_and_unsupported_layouts() {
        MediaKind::Qcow2.check_prefix(&header()).unwrap();
        for (offset, value) in [(8, 1), (16, 1), (32, 1), (60, 1), (72, 4), (88, 2)] {
            let mut h = header();
            h[offset] = value;
            assert!(
                MediaKind::Qcow2.check_prefix(&h).is_err(),
                "offset {offset}"
            );
        }
        assert!(MediaKind::Qcow2.check_prefix(b"QFI\xfb").is_err());
        assert!(MediaKind::Qcow2.check_prefix(&[0; 104]).is_err());
        // RAW never asks QEMU to interpret headers as a different disk format.
        MediaKind::Raw.check_prefix(b"QFI\xfb").unwrap();
    }
}
