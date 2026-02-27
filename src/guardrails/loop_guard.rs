const DEFAULT_EDIT_THRESHOLD: u32 = 25;

pub fn detect_loop(edits: u32) -> bool {
    detect_loop_with_threshold(edits, DEFAULT_EDIT_THRESHOLD)
}

pub fn detect_loop_with_threshold(edits: u32, threshold: u32) -> bool {
    edits >= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_loop_false_below_threshold() {
        assert!(!detect_loop_with_threshold(4, 5));
    }

    #[test]
    fn test_detect_loop_true_at_threshold() {
        assert!(detect_loop_with_threshold(5, 5));
    }
}
