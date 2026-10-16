//! QSSTV's `image1` … `imageN` rotation, as a plan of FTP operations.
//!
//! Kept free of any connection so the arithmetic can be tested. The newest
//! picture is always `image1`; every existing `image{i}` moves to
//! `image{i+1}`; whatever was at `imageN` falls off the end.
//!
//! QSSTV renames onto the last file and lets the server overwrite it. Not
//! every server does (IIS refuses a rename onto an existing name), so the
//! oldest is deleted first instead.

/// One step on the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Delete(String),
    Rename(String, String),
}

/// The name a rotated picture has at position `i` (1-based).
pub fn slot_name(i: u16, ext: &str) -> String {
    format!("image{i}{ext}")
}

/// The operations that make room for a new `image1`, given what is in the
/// directory now. `ext` is either empty or starts with a dot.
pub fn plan(listing: &[String], keep: u16, ext: &str) -> Vec<Op> {
    let keep = keep.max(1);
    // NLST may answer with paths rather than bare names on some servers.
    let has = |name: &str| listing.iter().any(|l| l.rsplit('/').next() == Some(name));
    let mut ops = Vec::new();
    let last = slot_name(keep, ext);
    if has(&last) {
        ops.push(Op::Delete(last));
    }
    for i in (2..=keep).rev() {
        let from = slot_name(i - 1, ext);
        if has(&from) {
            ops.push(Op::Rename(from, slot_name(i, ext)));
        }
    }
    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn an_empty_directory_needs_nothing() {
        assert!(plan(&[], 30, ".png").is_empty());
    }

    #[test]
    fn everything_moves_down_one_and_the_oldest_goes() {
        let ops = plan(&names(&["image1.png", "image2.png", "image3.png", "x.png"]), 3, ".png");
        assert_eq!(
            ops,
            vec![
                Op::Delete("image3.png".into()),
                Op::Rename("image2.png".into(), "image3.png".into()),
                Op::Rename("image1.png".into(), "image2.png".into()),
            ]
        );
    }

    #[test]
    fn gaps_are_left_alone() {
        let ops = plan(&names(&["image1", "image3"]), 5, "");
        assert_eq!(
            ops,
            vec![
                Op::Rename("image3".into(), "image4".into()),
                Op::Rename("image1".into(), "image2".into()),
            ]
        );
    }

    #[test]
    fn keeping_one_replaces_it() {
        assert_eq!(plan(&names(&["image1.jpg"]), 1, ".jpg"), vec![Op::Delete("image1.jpg".into())]);
    }

    #[test]
    fn extensions_must_match() {
        // QSSTV's bare names are not ours when an extension is on.
        assert!(plan(&names(&["image1", "image2"]), 30, ".png").is_empty());
    }

    #[test]
    fn listings_with_paths_are_understood() {
        let ops = plan(&names(&["pics/image1.png"]), 30, ".png");
        assert_eq!(ops, vec![Op::Rename("image1.png".into(), "image2.png".into())]);
    }
}
