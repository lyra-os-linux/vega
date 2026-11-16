//! Perfis do sistema (vegad#122, vegad#123). O vegad aplica e desfaz perfis
//! fixos (`org.lyraos.Vega1.Profiles`); aqui só se lê o estado e se pede a
//! mudança. O "modo estúdio" da seção Música (vega#217) liga
//! `audio-lowlatency` e, se o usuário quiser, `audio-lowlatency-kernel`.

use lyra_vega_dbus::{ProfileEvent, ProfileEventStream, ProfilesClient, SystemProfile};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::{CmdResult, State, err};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Profile {
    id: String,
    enabled: bool,
    restart_pending: bool,
}

impl From<SystemProfile> for Profile {
    fn from(profile: SystemProfile) -> Self {
        Self {
            id: profile.id,
            enabled: profile.enabled,
            restart_pending: profile.restart_pending,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Event {
    Progress {
        job: u32,
        id: String,
        percent: u32,
        message: String,
    },
    Finished {
        job: u32,
        id: String,
        result: String,
        detail: String,
    },
}

#[tauri::command]
pub async fn profiles_list(state: State<'_>) -> CmdResult<Vec<Profile>> {
    state
        .dbus()
        .await?
        .profiles()
        .list_system_profiles()
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(err)
}

/// Assina os sinais antes de pedir a mudança, para não perder o fim de um
/// perfil que não tinha nada a fazer.
#[tauri::command]
pub async fn profiles_set(
    app: AppHandle,
    state: State<'_>,
    id: String,
    enabled: bool,
) -> CmdResult<u32> {
    let profiles = state.dbus().await?.profiles();
    let events = profiles.subscribe().await.map_err(err)?;
    let job = profiles
        .set_system_profile(&id, enabled)
        .await
        .map_err(err)?;
    tauri::async_runtime::spawn(follow(app, events, job, id));
    Ok(job)
}

async fn follow(app: AppHandle, mut events: ProfileEventStream, job: u32, id: String) {
    loop {
        let event = match events.next().await {
            Ok(ProfileEvent::Progress(p)) if p.job_id == job => Event::Progress {
                job,
                id: p.id,
                percent: p.percent,
                message: p.message,
            },
            Ok(ProfileEvent::Finished(f)) if f.job_id == job => Event::Finished {
                job,
                id: f.id,
                result: f.result,
                detail: f.detail,
            },
            Ok(_) => continue,
            Err(error) => Event::Finished {
                job,
                id: id.clone(),
                result: "failed".into(),
                detail: err(error),
            },
        };
        let done = matches!(event, Event::Finished { .. });
        let _ = app.emit("profile-event", event);
        if done {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_rows_serialize_for_the_ui() {
        let profile = Profile {
            id: "audio-lowlatency".into(),
            enabled: true,
            restart_pending: true,
        };
        let json = serde_json::to_value(&profile).unwrap();
        assert_eq!(json["id"], "audio-lowlatency");
        assert_eq!(json["restart_pending"], true);
        let finished = serde_json::to_value(Event::Finished {
            job: 3,
            id: "audio-lowlatency".into(),
            result: "ok".into(),
            detail: String::new(),
        })
        .unwrap();
        assert_eq!(finished["type"], "finished");
    }
}
