//! Text deltas never expose internal-operation payloads to the chat renderer.
#[derive(Default)]
pub(super) struct TextFilter {
    prefix: String,
    decided: bool,
    internal: bool,
}
impl TextFilter {
    pub fn push(&mut self, delta: &str) -> Option<String> {
        if self.decided {
            return (!self.internal).then(|| delta.to_owned());
        }
        self.prefix.push_str(delta);
        let value = self.prefix.trim_start();
        if value.starts_with("<internal_action>") {
            self.decided = true;
            self.internal = true;
            self.prefix.clear();
            return None;
        }
        if "<internal_action>".starts_with(value) {
            return None;
        }
        self.decided = true;
        Some(std::mem::take(&mut self.prefix))
    }
    pub fn finish(&mut self) -> Option<String> {
        if self.internal || self.prefix.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.prefix))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_internal_prefix_is_suppressed_but_ordinary_text_is_not_delayed() {
        let mut filter = TextFilter::default();
        for chunk in [
            " \n<inte",
            "rnal_action",
            ">{\"operation\":",
            "\"task.list\"}</internal_action>",
        ] {
            assert!(filter.push(chunk).is_none());
        }
        assert!(filter.finish().is_none());
        let mut filter = TextFilter::default();
        assert!(filter.push(" <").is_none());
        assert_eq!(filter.push("hello"), Some(" <hello".into()));
        assert_eq!(filter.push(" world"), Some(" world".into()));
    }
}
