//! P10 file/media admission. Interpretation runs later in a sandbox; this layer
//! establishes ownership, immutable provenance, and bounded formats first.
use crate::assistants::AgentId;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const MAX_MEDIA_BYTES: usize = 250 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaArtifact {
    pub owner: AgentId,
    pub task_id: String,
    pub filename: String,
    pub mime_type: String,
    pub sha256: String,
    pub size: usize,
    pub source: String,
}
pub fn admit_upload(
    owner: AgentId,
    task_id: &str,
    filename: &str,
    mime_type: &str,
    bytes: &[u8],
    source: &str,
) -> Result<MediaArtifact> {
    if task_id.trim().is_empty()
        || source.trim().is_empty()
        || filename.is_empty()
        || filename.len() > 200
        || filename.contains(['/', '\\'])
    {
        bail!("Invalid media provenance or filename");
    }
    if bytes.is_empty() || bytes.len() > MAX_MEDIA_BYTES {
        bail!("Media artifact exceeds the phase limit");
    }
    if !matches!(
        mime_type,
        "image/jpeg"
            | "image/png"
            | "image/webp"
            | "application/pdf"
            | "video/mp4"
            | "video/webm"
            | "audio/mpeg"
            | "audio/wav"
            | "text/plain"
    ) {
        bail!("Unsupported media type");
    }
    Ok(MediaArtifact {
        owner,
        task_id: task_id.into(),
        filename: filename.into(),
        mime_type: mime_type.into(),
        sha256: hex::encode(Sha256::digest(bytes)),
        size: bytes.len(),
        source: source.into(),
    })
}

/// Trusted decoder commands, with bounded output and a networkless container.
pub fn processing_command(kind: &str) -> Result<String> {
    let decode=match kind {
        "video"=>r"python3 - <<'PY'
import json,math,subprocess
probe=subprocess.run(['ffprobe','-v','error','-show_entries','format=duration','-of','json','/inputs/input'],capture_output=True,check=True,timeout=10)
assert len(probe.stdout)<16000
duration=float(json.loads(probe.stdout)['format']['duration'])
assert math.isfinite(duration) and 0<duration<=7200
subprocess.run(['ffmpeg','-nostdin','-v','error','-threads','1','-i','/inputs/input','-vf',f'fps={6/max(duration,1)},scale=240:160:force_original_aspect_ratio=decrease,pad=240:160:(ow-iw)/2:(oh-ih)/2,tile=3x2','-frames:v','1','-q:v','12','/workspace/preview.jpg'],check=True)
PY",
        "image"=>"python3 -c 'from PIL import Image; Image.MAX_IMAGE_PIXELS=20000000; im=Image.open(\"/inputs/input\"); im.thumbnail((720,480)); im.convert(\"RGB\").save(\"/workspace/preview.jpg\",quality=45)'",
        _=>bail!("Only bounded image/video preview workflows are registered"),
    };
    Ok(format!("set -e\n{decode}\npython3 -c 'import base64,json,os; p=\"/workspace/preview.jpg\"; assert os.path.getsize(p)<45000; print(json.dumps({{\"artifact_base64\":base64.b64encode(open(p,\"rb\").read()).decode()}}))'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn media_is_owned_and_bounded() {
        let a = admit_upload(
            AgentId::Harley,
            "task-1",
            "clip.mp4",
            "video/mp4",
            b"video",
            "user_upload",
        )
        .unwrap();
        assert_eq!(a.sha256.len(), 64);
        assert!(admit_upload(
            AgentId::Harley,
            "task-1",
            "../x",
            "video/mp4",
            b"x",
            "user_upload"
        )
        .is_err());
    }
}
