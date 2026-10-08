/// Collects debug labels in the original standalone prototype.
///
/// This recorder is not device-bound. Use [`crate::CommandEncoder`] to record
/// actual copy descriptions. Markers are text, not graphics operations.
#[derive(Debug, Default)]
pub struct CommandRecorder {
    debug_markers: Vec<String>,
}

impl CommandRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores its own copy of the label.
    pub fn insert_debug_marker(&mut self, label: &str) {
        self.debug_markers.push(label.to_owned());
    }

    /// Consumes the recorder without executing anything.
    ///
    /// ```compile_fail,E0382
    /// use expansex_core::CommandRecorder;
    /// let mut recorder = CommandRecorder::new();
    /// let markers = recorder.finish();
    /// recorder.insert_debug_marker("too late");
    /// ```
    pub fn finish(self) -> DebugMarkerBuffer {
        DebugMarkerBuffer {
            debug_markers: self.debug_markers,
        }
    }
}

/// Finished prototype labels, distinct from device-bound executable descriptions.
#[derive(Debug)]
pub struct DebugMarkerBuffer {
    debug_markers: Vec<String>,
}

impl DebugMarkerBuffer {
    pub fn debug_markers(&self) -> &[String] {
        &self.debug_markers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finished_markers_keep_labels_after_the_original_is_dropped() {
        let mut recorder = CommandRecorder::new();
        let label = String::from("prepare triangle");
        recorder.insert_debug_marker(&label);
        drop(label);
        recorder.insert_debug_marker("draw triangle (planned)");
        let markers = recorder.finish();
        assert_eq!(
            markers.debug_markers(),
            &["prepare triangle", "draw triangle (planned)"]
        );
    }

    #[test]
    fn an_empty_recording_can_be_finished() {
        assert!(CommandRecorder::new().finish().debug_markers().is_empty());
    }
}
