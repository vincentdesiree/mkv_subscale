#[derive(Debug, PartialEq, Eq)]
pub enum SubtitleOutcome {
    /// The subtitle was already compliant, no changes needed.
    Unchanged,
    /// The subtitle was modified and contains the updated ASS content.
    Modified { new_content: String },
}

#[must_use]
pub fn process_ass_content(content: &str) -> SubtitleOutcome {
    let clean_content = content.trim_start_matches('\u{feff}');
    let mut lines: Vec<String> = clean_content.lines().map(String::from).collect();

    let mut in_script_info = false;
    let mut found_flag = false;
    let mut modified = false;

    for line in &mut lines {
        let trimmed = line.trim();

        if trimmed.eq_ignore_ascii_case("[Script Info]") {
            in_script_info = true;
            continue;
        }

        if in_script_info && trimmed.starts_with('[') && trimmed.ends_with(']') {
            break;
        }

        if in_script_info
            && let Some((key, val)) = trimmed.split_once(':')
            && key.trim().eq_ignore_ascii_case("ScaledBorderAndShadow")
        {
            found_flag = true;
            let val_clean = val.trim().to_lowercase();

            if val_clean != "yes" && val_clean != "1" {
                *line = "ScaledBorderAndShadow: yes".to_string();
                modified = true;
            }
            break;
        }
    }

    if !found_flag
        && let Some(pos) = lines
            .iter()
            .position(|l| l.trim().eq_ignore_ascii_case("[Script Info]"))
    {
        lines.insert(pos + 1, "ScaledBorderAndShadow: yes".to_string());
        modified = true;
    }

    if modified {
        let mut result = lines.join("\n");
        if content.ends_with('\n') {
            result.push('\n');
        }
        SubtitleOutcome::Modified {
            new_content: result,
        }
    } else {
        SubtitleOutcome::Unchanged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_already_compliant_with_yes() {
        let content = "\
[Script Info]
Title: English
ScaledBorderAndShadow: yes
PlayResX: 1920
PlayResY: 1080
";
        assert_eq!(process_ass_content(content), SubtitleOutcome::Unchanged);
    }

    #[test]
    fn test_already_compliant_with_numeric_one() {
        let content = "\
[Script Info]
ScaledBorderAndShadow: 1
PlayResX: 1920
";
        assert_eq!(process_ass_content(content), SubtitleOutcome::Unchanged);
    }

    #[test]
    fn test_already_compliant_case_insensitive_and_whitespace() {
        let content = "[Script Info]\nScaledBorderAndShadow:  YES \n";
        assert_eq!(process_ass_content(content), SubtitleOutcome::Unchanged);
    }

    #[test]
    fn test_modifies_no_to_yes() {
        let content = "\
[Script Info]
Title: Sample
ScaledBorderAndShadow: no
PlayResX: 1280
";
        let expected = "\
[Script Info]
Title: Sample
ScaledBorderAndShadow: yes
PlayResX: 1280
";
        let outcome = process_ass_content(content);
        assert_eq!(
            outcome,
            SubtitleOutcome::Modified {
                new_content: expected.to_string()
            }
        );
    }

    #[test]
    fn test_modifies_zero_to_yes() {
        let content = "[Script Info]\nScaledBorderAndShadow: 0\n";
        let expected = "[Script Info]\nScaledBorderAndShadow: yes\n";

        let outcome = process_ass_content(content);
        assert_eq!(
            outcome,
            SubtitleOutcome::Modified {
                new_content: expected.to_string()
            }
        );
    }

    #[test]
    fn test_inserts_flag_when_missing_in_script_info() {
        let content = "\
[Script Info]
Title: Anime Episode
PlayResX: 1920

[V4+ Styles]
Format: Name, Fontname
";
        let expected = "\
[Script Info]
ScaledBorderAndShadow: yes
Title: Anime Episode
PlayResX: 1920

[V4+ Styles]
Format: Name, Fontname
";
        let outcome = process_ass_content(content);
        assert_eq!(
            outcome,
            SubtitleOutcome::Modified {
                new_content: expected.to_string()
            }
        );
    }

    #[test]
    fn test_strips_bom_and_preserves_trailing_newline() {
        let content_with_bom = "\u{feff}[Script Info]\nScaledBorderAndShadow: no\n";
        let expected = "[Script Info]\nScaledBorderAndShadow: yes\n";

        let outcome = process_ass_content(content_with_bom);
        assert_eq!(
            outcome,
            SubtitleOutcome::Modified {
                new_content: expected.to_string()
            }
        );
    }

    #[test]
    fn test_no_script_info_section_returns_unchanged() {
        let content = "\
[V4+ Styles]
Format: Name, Fontname
Style: Default,Arial,20
";
        assert_eq!(process_ass_content(content), SubtitleOutcome::Unchanged);
    }
}
