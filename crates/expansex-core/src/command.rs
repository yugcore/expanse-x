/// Collects command descriptions on the CPU.
///
/// Only debug markers are supported by this initial design prototype.
/// A marker is a label for inspecting a command sequence, not a GPU operation.
#[derive(Debug, Default)]
pub struct CommandRecorder {
    debug_markers: Vec<String>,
}

impl CommandRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores its own copy of a label, so the caller may release the original.
    pub fn insert_debug_marker(&mut self, label: &str) {
        self.debug_markers.push(label.to_owned());
    }

    /// Consumes the recorder and produces a finished, immutable description.
    ///
    /// Using the recorder afterward is a compile-time error:
    ///
    /// ```compile_fail,E0382
    /// use expansex_core::CommandRecorder;
    /// let mut recorder = CommandRecorder::new();
    /// let commands = recorder.finish();
    /// recorder.insert_debug_marker("too late");
    /// ```
    pub fn finish(self) -> CommandBuffer {
        CommandBuffer {
            debug_markers: self.debug_markers,
        }
    }
}

/// A finished CPU-side command description, with no backend allocation yet.
///
/// No submission API exists in this draft. Finishing does not execute work.
#[derive(Debug)]
pub struct CommandBuffer {
    debug_markers: Vec<String>,
}

impl CommandBuffer {
    /// Reads the recorded labels in insertion order.
    pub fn debug_markers(&self) -> &[String] {
        &self.debug_markers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finished_commands_keep_labels_after_the_original_is_dropped() {
        let mut recorder = CommandRecorder::new();
        let label = String::from("prepare triangle");
        recorder.insert_debug_marker(&label);
        drop(label);
        recorder.insert_debug_marker("draw triangle (planned)");
        let commands = recorder.finish();
        assert_eq!(
            commands.debug_markers(),
            &["prepare triangle", "draw triangle (planned)"]
        );
    }

    #[test]
    fn an_empty_recording_can_be_finished() {
        assert!(CommandRecorder::new().finish().debug_markers().is_empty());
    }
}
