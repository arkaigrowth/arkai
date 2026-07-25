//! Voice source configuration for multi-source scanning.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceSourceHandler {
    VoicememosContainer,
    PlainAudioDir,
    PrediarizedTranscript,
}

impl VoiceSourceHandler {
    fn parse(raw: &str) -> Result<Option<Self>> {
        match raw {
            "voicememos_container" => Ok(Some(Self::VoicememosContainer)),
            "plain_audio_dir" => Ok(Some(Self::PlainAudioDir)),
            "prediarized_transcript" => Ok(Some(Self::PrediarizedTranscript)),
            "zoom_meeting_folder" => anyhow::bail!("zoom sources are V2, run Zoom manually"),
            other => {
                tracing::warn!(
                    handler = other,
                    "skipping voice source with unknown handler"
                );
                Ok(None)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceSource {
    pub name: String,
    pub path: PathBuf,
    pub handler: VoiceSourceHandler,
    pub private: bool,
    pub category: Option<String>,
    pub extensions: Option<Vec<String>>,
    pub min_age_secs: Option<u64>,
}

impl VoiceSource {
    pub fn default_extensions(&self) -> Vec<String> {
        match self.handler {
            VoiceSourceHandler::VoicememosContainer => vec!["m4a".to_string(), "qta".to_string()],
            VoiceSourceHandler::PlainAudioDir => ["m4a", "mp3", "wav", "flac", "mp4", "mov"]
                .iter()
                .map(|ext| (*ext).to_string())
                .collect(),
            VoiceSourceHandler::PrediarizedTranscript => {
                vec!["txt".to_string(), "md".to_string()]
            }
        }
    }

    pub fn effective_extensions(&self) -> Vec<String> {
        self.extensions
            .clone()
            .filter(|extensions| !extensions.is_empty())
            .unwrap_or_else(|| self.default_extensions())
    }
}

#[derive(Debug, Deserialize)]
struct VoiceSourcesFile {
    #[serde(default)]
    sources: Vec<RawVoiceSource>,
}

#[derive(Debug, Deserialize)]
struct RawVoiceSource {
    name: String,
    path: String,
    handler: String,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    extensions: Option<Vec<String>>,
    #[serde(default)]
    min_age_secs: Option<u64>,
}

pub fn load_voice_sources() -> Result<Vec<VoiceSource>> {
    let home = crate::config::arkai_home()?;
    load_voice_sources_from_home(&home)
}

pub(crate) fn load_voice_sources_from_home(home: &Path) -> Result<Vec<VoiceSource>> {
    let path = home.join("voice_sources.yaml");
    if !path.exists() {
        return Ok(default_sources());
    }

    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read voice sources file: {}", path.display()))?;
    let parsed: VoiceSourcesFile = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse voice sources file: {}", path.display()))?;

    let mut sources = Vec::new();
    for raw in parsed.sources {
        let handler = match VoiceSourceHandler::parse(&raw.handler) {
            Ok(Some(handler)) => handler,
            Ok(None) => continue,
            Err(error) if raw.handler == "zoom_meeting_folder" => return Err(error),
            Err(error) => return Err(error).with_context(|| format!("source '{}'", raw.name)),
        };

        sources.push(VoiceSource {
            name: raw.name,
            path: expand_tilde(&raw.path),
            handler,
            private: raw.private,
            category: raw.category,
            extensions: raw.extensions,
            min_age_secs: raw.min_age_secs,
        });
    }

    Ok(sources)
}

pub fn default_sources() -> Vec<VoiceSource> {
    vec![
        VoiceSource {
            name: "voicememos".to_string(),
            path: expand_tilde(
                "~/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings",
            ),
            handler: VoiceSourceHandler::VoicememosContainer,
            private: false,
            category: None,
            extensions: None,
            min_age_secs: None,
        },
        VoiceSource {
            name: "transcribex-media".to_string(),
            path: expand_tilde("~/Documents/transcribex/media"),
            handler: VoiceSourceHandler::PlainAudioDir,
            private: false,
            category: None,
            extensions: None,
            min_age_secs: None,
        },
    ]
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(path));
    }

    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }

    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_sources(home: &Path, yaml: &str) {
        std::fs::create_dir_all(home).unwrap();
        std::fs::write(home.join("voice_sources.yaml"), yaml).unwrap();
    }

    #[test]
    fn test_absent_file_uses_defaults() {
        let temp = TempDir::new().unwrap();

        let sources = load_voice_sources_from_home(temp.path()).unwrap();

        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].name, "voicememos");
        assert_eq!(sources[0].handler, VoiceSourceHandler::VoicememosContainer);
        assert_eq!(sources[1].name, "transcribex-media");
        assert_eq!(sources[1].handler, VoiceSourceHandler::PlainAudioDir);
    }

    #[test]
    fn test_yaml_parse_full_source() {
        let temp = TempDir::new().unwrap();
        write_sources(
            temp.path(),
            r#"
sources:
  - name: private-export
    path: ~/Documents/exports
    handler: prediarized_transcript
    private: true
    category: private-notes
    extensions: [txt, md]
    min_age_secs: 45
"#,
        );

        let sources = load_voice_sources_from_home(temp.path()).unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "private-export");
        assert_eq!(
            sources[0].handler,
            VoiceSourceHandler::PrediarizedTranscript
        );
        assert!(sources[0].private);
        assert_eq!(sources[0].category.as_deref(), Some("private-notes"));
        assert_eq!(
            sources[0].extensions.as_ref().unwrap(),
            &vec!["txt".to_string(), "md".to_string()]
        );
        assert_eq!(sources[0].min_age_secs, Some(45));
        assert!(sources[0].path.is_absolute());
    }

    #[test]
    fn test_yaml_parse_minimal_source_defaults_optional_fields() {
        let temp = TempDir::new().unwrap();
        write_sources(
            temp.path(),
            r#"
sources:
  - name: media
    path: /tmp/media
    handler: plain_audio_dir
"#,
        );

        let sources = load_voice_sources_from_home(temp.path()).unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "media");
        assert_eq!(sources[0].path, PathBuf::from("/tmp/media"));
        assert_eq!(sources[0].handler, VoiceSourceHandler::PlainAudioDir);
        assert!(!sources[0].private);
        assert!(sources[0].category.is_none());
        assert!(sources[0].extensions.is_none());
        assert!(sources[0].min_age_secs.is_none());
    }

    #[test]
    fn test_unknown_handler_skips_source() {
        let temp = TempDir::new().unwrap();
        write_sources(
            temp.path(),
            r#"
sources:
  - name: bad
    path: /tmp/bad
    handler: made_up
  - name: good
    path: /tmp/good
    handler: plain_audio_dir
"#,
        );

        let sources = load_voice_sources_from_home(temp.path()).unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "good");
    }

    #[test]
    fn test_zoom_handler_errors_at_load() {
        let temp = TempDir::new().unwrap();
        write_sources(
            temp.path(),
            r#"
sources:
  - name: zoom
    path: /tmp/zoom
    handler: zoom_meeting_folder
"#,
        );

        let error = load_voice_sources_from_home(temp.path()).unwrap_err();

        assert!(error.to_string().contains("zoom sources are V2"));
    }

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~/Documents/transcribex/media");

        assert!(expanded.is_absolute());
        assert!(expanded.ends_with("Documents/transcribex/media"));
        assert_eq!(expand_tilde("/tmp/plain"), PathBuf::from("/tmp/plain"));
    }
}
