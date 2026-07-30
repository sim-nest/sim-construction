//! Reference-only office evidence composition for construction control.
//!
//! The bridge joins construction facts to existing office documents without
//! owning document bytes, an evidence store, or construction acceptance state.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod evidence;
