#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(clippy::get_first)]
#![allow(clippy::new_without_default)]
// Pattern contains a Regex which apparently has some interior mutability.
// That should however not influence our Ord and Eq implementations.
#![allow(clippy::mutable_key_type)]

const NON_JOURNALING_FILESYSTEMS: &[&str] = &["exfat", "ext2", "vfat"];

pub mod ui;
