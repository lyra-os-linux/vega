//! Ids das transações que o Vega roda na sessão do usuário (Flatpak do
//! usuário, Wine, firmware, npm, AppImage). Um contador só para todas, numa
//! faixa acima da do vegad: dois módulos com contadores próprios na mesma
//! faixa davam o mesmo id a transações simultâneas (vega#245).

use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_ID: AtomicU32 = AtomicU32::new(0x8000_0000);

/// Um id novo, nunca repetido nesta execução.
pub fn next() -> u32 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_from_every_thread_are_unique_and_above_the_daemon_range() {
        let ids: Vec<u32> = (0..8)
            .map(|_| std::thread::spawn(|| (0..1000).map(|_| next()).collect::<Vec<_>>()))
            .flat_map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(ids.iter().collect::<HashSet<_>>().len(), ids.len());
        assert!(ids.iter().all(|id| *id >= 0x8000_0000));
    }
}
