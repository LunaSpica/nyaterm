
use super::*;

#[test]
fn buffer_matches_report_column_ranges() {
    let output = "hello world\nfoo hello bar";
    let matches = terminal_buffer_matches(
        output,
        "hello",
        &TerminalSearchFlags {
            case_sensitive: false,
            regex: false,
            whole_word: false,
        },
        10,
    )
    .expect("matches");
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].line_index, 0);
    assert_eq!(matches[0].start_col, 0);
    assert_eq!(matches[0].end_col, 5);
    assert_eq!(matches[1].line_index, 1);
    assert_eq!(matches[1].start_col, 4);
    assert_eq!(matches[1].end_col, 9);
}
