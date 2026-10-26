//! Deliberate violations, one per entry in the lint table.
//!
//! Never fix anything in this file: each violation is the proof. A marker names
//! a lint and the line it applies to. `// expect: <lint>` means the lint must
//! fire there. `// quiet: <lint>` means the table allows the lint, so the code
//! below it must stay silent. A marker on a line of its own applies to the next
//! code line, so a line that trips several lints carries several markers.

use std::rc::Rc;

/// A parse failure.
#[derive(Debug)]
pub struct ParseError;

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("parse error")
    }
}

impl std::error::Error for ParseError {}

// ----------------------------------------------------------------- [lints.rust]

/// Reads `slot` without an inner `unsafe` block.
///
/// # Safety
///
/// `slot` must be valid for reads.
// expect: unsafe_code
pub unsafe fn read_slot(slot: *const u32) -> u32 {
    *slot // expect: unsafe_op_in_unsafe_fn
}

/// A public type without `Debug`.
pub struct Silent; // expect: missing_debug_implementations

mod internal {
    /// An item that should not be `pub`.
    pub fn unused_export() {} // expect: unreachable_pub
}

pub fn undocumented() {} // expect: missing_docs

/// Writes to a formatter without naming its lifetime.
///
/// # Errors
///
/// Returns an error if the formatter fails.
// expect: rust_2018_idioms
pub fn write_hidden(f: &mut std::fmt::Formatter) -> std::fmt::Result {
    f.write_str("hidden")
}

// -------------------------------------------------------------- [lints.rustdoc]

#[doc = "Links to [`Missing`], which does not exist."] // expect: rustdoc::broken_intra_doc_links
pub fn broken_link() {}

#[doc = "Links to [`internal::unused_export`], which is private."] // expect: rustdoc::private_intra_doc_links
pub fn private_link() {}

// ------------------------------------------------------------ clippy groups

/// Returns with a needless `return`, a lint only the `all` group turns on.
pub fn early_return(n: u32) -> u32 {
    return n; // expect: clippy::all
}

/// Calls into the private module with no semicolon, a pedantic lint.
pub fn touch_internal() {
    internal::unused_export() // expect: clippy::pedantic
}

// ------------------------------------------------------------------- panics

/// Unwraps `value`.
///
/// # Panics
///
/// Panics if `value` is `None`.
pub fn take_or_panic(value: Option<u32>) -> u32 {
    value.unwrap() // expect: clippy::unwrap_used
}

/// Expects `value`.
///
/// # Panics
///
/// Panics if `value` is `None`.
pub fn take_or_expect(value: Option<u32>) -> u32 {
    value.expect("value is present") // expect: clippy::expect_used
}

/// Panics on purpose.
///
/// # Panics
///
/// Always.
pub fn boom() {
    panic!("boom"); // expect: clippy::panic
}

/// Leaves a `todo!` behind.
///
/// # Panics
///
/// Always, until written.
pub fn unfinished() {
    todo!() // expect: clippy::todo
}

/// Leaves an `unimplemented!` behind.
///
/// # Panics
///
/// Always.
pub fn not_written() {
    unimplemented!() // expect: clippy::unimplemented
}

/// Claims a branch cannot run.
///
/// # Panics
///
/// When `n` is zero.
pub fn never_zero(n: u32) -> u32 {
    if n == 0 {
        unreachable!("n is never zero") // expect: clippy::unreachable
    }
    n
}

/// Indexes a slice without a check.
///
/// # Panics
///
/// When `values` is empty.
pub fn first(values: &[u32]) -> u32 {
    values[0] // expect: clippy::indexing_slicing
}

/// Asserts inside a function that returns `Result`.
///
/// # Errors
///
/// Never: it panics instead.
///
/// # Panics
///
/// When `input` is empty.
// expect: clippy::panic_in_result_fn
pub fn parse_or_panic(input: &str) -> Result<usize, ParseError> {
    assert!(!input.is_empty(), "input is empty");
    Ok(input.len())
}

/// Asserts without saying so in the docs.
// expect: clippy::missing_panics_doc
pub fn check_positive(n: i32) {
    assert!(n > 0, "n must be positive, got {n}");
}

/// Fails without saying when in the docs.
// expect: clippy::missing_errors_doc
pub fn parse_input(_input: &str) -> Result<usize, ParseError> {
    Err(ParseError)
}

// ------------------------------------------------------------- suppressions

/// Suppresses a lint with `allow` and no reason.
// expect: clippy::allow_attributes
// expect: clippy::allow_attributes_without_reason
#[allow(clippy::needless_pass_by_value)]
pub fn takes_owned(text: String) -> usize {
    text.len()
}

// ------------------------------------------------------------------- unsafe

/// Reads a local through a raw pointer.
pub fn read_local() -> u32 {
    let value = 1_u32;
    let slot = &raw const value;
    // expect: unsafe_code
    // expect: clippy::undocumented_unsafe_blocks
    unsafe { *slot }
}

/// Does two unsafe operations in one block.
///
/// # Safety
///
/// `a` and `b` must be valid for reads.
// expect: unsafe_code
pub unsafe fn read_two(a: *const u32, b: *const u32) -> u32 {
    // SAFETY: the caller guarantees both pointers are valid for reads.
    // expect: unsafe_code
    // expect: clippy::multiple_unsafe_ops_per_block
    unsafe { *a + *b }
}

// ---------------------------------------------------------------- API shape

/// Takes one `bool`. The root `clippy.toml` sets the limit to zero.
// expect: clippy::fn_params_excessive_bools
pub fn render(verbose: bool) -> &'static str {
    if verbose { "long" } else { "short" }
}

/// Carries four boolean flags.
#[derive(Debug)]
// expect: clippy::struct_excessive_bools
pub struct Flags {
    /// Flag a.
    pub a: bool,
    /// Flag b.
    pub b: bool,
    /// Flag c.
    pub c: bool,
    /// Flag d.
    pub d: bool,
}

/// Clones an `Rc` with method syntax.
pub fn share(text: &Rc<str>) -> Rc<str> {
    text.clone() // expect: clippy::clone_on_ref_ptr
}

/// Clones a local `String` instead of moving it.
pub fn clone_text() -> String {
    let text = String::from("label");
    text.clone() // expect: clippy::redundant_clone
}

/// Holds an `Rc<String>`.
#[derive(Debug)]
pub struct SharedText {
    text: Rc<String>, // expect: clippy::rc_buffer
}

impl SharedText {
    /// Wraps a `String` in `Rc`.
    pub fn from_text(text: String) -> Self {
        Self {
            text: Rc::new(text),
        }
    }

    /// Returns the wrapped text.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

// ------------------------------------------------------------------ strings

/// Concatenates with `+`.
pub fn join_label(prefix: String) -> String {
    prefix + "-suffix" // expect: clippy::string_add
}

/// Slices a string by bytes, which panics inside a multi-byte character.
pub fn head(text: &str) -> &str {
    &text[..2] // expect: clippy::string_slice
}

/// Turns a `&str` into a `String` with `to_string`.
pub fn owned_label() -> String {
    "label".to_string() // expect: clippy::str_to_string
}

/// Imports a module with a glob.
pub fn glob_sink() {
    use std::io::*; // expect: clippy::wildcard_imports
    let mut buf = [0_u8; 1];
    drop(empty().read(&mut buf));
}

// ------------------------------------------------------ leftovers and output

/// Leaves a `dbg!` behind.
pub fn traced(n: u32) -> u32 {
    dbg!(n) // expect: clippy::dbg_macro
}

/// Prints to standard output from library code.
pub fn shout() {
    println!("hello"); // expect: clippy::print_stdout
}

/// Prints to standard error from library code.
pub fn complain() {
    eprintln!("oops"); // expect: clippy::print_stderr
}

// -------------------------------------------------------------------- casts

/// Narrows a `u64` with `as`.
pub fn narrow(n: u64) -> u32 {
    // expect: clippy::as_conversions
    // expect: clippy::cast_possible_truncation
    n as u32
}

/// Reinterprets a signed value as unsigned with `as`.
pub fn unsign(n: i32) -> u32 {
    // expect: clippy::as_conversions
    // expect: clippy::cast_sign_loss
    n as u32
}

// -------------------------------------------------------------------- async

/// Holds 18 KB across an await point, which makes a large future.
pub async fn holds_large_buffers() -> usize {
    let first = [1_u8; 9_000];
    let second = [2_u8; 9_000];
    std::future::ready(()).await;
    first.len().saturating_add(second.len())
}

/// Awaits the large future by value.
pub async fn awaits_large_future() -> usize {
    holds_large_buffers().await // expect: clippy::large_futures
}

/// Is `async` without awaiting anything.
// expect: clippy::unused_async
pub async fn no_await() -> u32 {
    1
}

// ------------------------------------------------- disallowed methods, types

// Every entry in the root clippy.toml has a violation here, spelled with its
// full path so scripts/check-lints.sh can find it. Only the clock, config,
// outbound and path modules may suppress these, once each.

/// Reads the system time directly instead of through `Clock`.
pub fn system_time() -> std::time::SystemTime {
    std::time::SystemTime::now() // expect: clippy::disallowed_methods
}

/// Reads the monotonic clock directly instead of through `Clock`.
pub fn instant() -> std::time::Instant {
    std::time::Instant::now() // expect: clippy::disallowed_methods
}

/// Reads UTC time from `chrono` directly instead of through `Clock`.
pub fn utc_time() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now() // expect: clippy::disallowed_methods
}

/// Reads local time from `chrono` directly instead of through `Clock`.
pub fn local_time() -> chrono::DateTime<chrono::Local> {
    chrono::Local::now() // expect: clippy::disallowed_methods
}

/// Reads the environment directly instead of through `Config`.
pub fn site_from_env() -> Option<String> {
    std::env::var("KHALID_SITE").ok() // expect: clippy::disallowed_methods
}

/// Reads the environment as an `OsString` instead of through `Config`.
pub fn site_from_env_os() -> Option<std::ffi::OsString> {
    std::env::var_os("KHALID_SITE") // expect: clippy::disallowed_methods
}

/// Joins an untrusted segment onto a path instead of through `SitePath`.
pub fn join_untrusted(root: &std::path::Path, segment: &str) -> std::path::PathBuf {
    std::path::Path::join(root, segment) // expect: clippy::disallowed_methods
}

/// Ends the process instead of returning an error to `main`.
pub fn quit() -> ! {
    std::process::exit(1) // expect: clippy::disallowed_methods
}

/// Keeps counts in a map whose order is random.
#[derive(Debug)]
pub struct WordCounts {
    /// Words and how often each appears.
    pub counts: std::collections::HashMap<String, u32>, // expect: clippy::disallowed_types
}

// --------------------------------------------------------------------- size

/// Takes seven parameters, one more than the limit.
// expect: clippy::too_many_arguments
pub fn seven(
    first: u8,
    second: u8,
    third: u8,
    fourth: u8,
    fifth: u8,
    sixth: u8,
    seventh: u8,
) -> u32 {
    [first, second, third, fourth, fifth, sixth, seventh]
        .into_iter()
        .map(u32::from)
        .sum()
}

/// Runs past the 80-line limit.
// expect: clippy::too_many_lines
pub fn long_function() -> u32 {
    let mut total = 0_u32;
    total = total.wrapping_add(1);
    total = total.wrapping_add(2);
    total = total.wrapping_add(3);
    total = total.wrapping_add(4);
    total = total.wrapping_add(5);
    total = total.wrapping_add(6);
    total = total.wrapping_add(7);
    total = total.wrapping_add(8);
    total = total.wrapping_add(9);
    total = total.wrapping_add(10);
    total = total.wrapping_add(11);
    total = total.wrapping_add(12);
    total = total.wrapping_add(13);
    total = total.wrapping_add(14);
    total = total.wrapping_add(15);
    total = total.wrapping_add(16);
    total = total.wrapping_add(17);
    total = total.wrapping_add(18);
    total = total.wrapping_add(19);
    total = total.wrapping_add(20);
    total = total.wrapping_add(21);
    total = total.wrapping_add(22);
    total = total.wrapping_add(23);
    total = total.wrapping_add(24);
    total = total.wrapping_add(25);
    total = total.wrapping_add(26);
    total = total.wrapping_add(27);
    total = total.wrapping_add(28);
    total = total.wrapping_add(29);
    total = total.wrapping_add(30);
    total = total.wrapping_add(31);
    total = total.wrapping_add(32);
    total = total.wrapping_add(33);
    total = total.wrapping_add(34);
    total = total.wrapping_add(35);
    total = total.wrapping_add(36);
    total = total.wrapping_add(37);
    total = total.wrapping_add(38);
    total = total.wrapping_add(39);
    total = total.wrapping_add(40);
    total = total.wrapping_add(41);
    total = total.wrapping_add(42);
    total = total.wrapping_add(43);
    total = total.wrapping_add(44);
    total = total.wrapping_add(45);
    total = total.wrapping_add(46);
    total = total.wrapping_add(47);
    total = total.wrapping_add(48);
    total = total.wrapping_add(49);
    total = total.wrapping_add(50);
    total = total.wrapping_add(51);
    total = total.wrapping_add(52);
    total = total.wrapping_add(53);
    total = total.wrapping_add(54);
    total = total.wrapping_add(55);
    total = total.wrapping_add(56);
    total = total.wrapping_add(57);
    total = total.wrapping_add(58);
    total = total.wrapping_add(59);
    total = total.wrapping_add(60);
    total = total.wrapping_add(61);
    total = total.wrapping_add(62);
    total = total.wrapping_add(63);
    total = total.wrapping_add(64);
    total = total.wrapping_add(65);
    total = total.wrapping_add(66);
    total = total.wrapping_add(67);
    total = total.wrapping_add(68);
    total = total.wrapping_add(69);
    total = total.wrapping_add(70);
    total = total.wrapping_add(71);
    total = total.wrapping_add(72);
    total = total.wrapping_add(73);
    total = total.wrapping_add(74);
    total = total.wrapping_add(75);
    total = total.wrapping_add(76);
    total = total.wrapping_add(77);
    total = total.wrapping_add(78);
    total = total.wrapping_add(79);
    total = total.wrapping_add(80);
    total = total.wrapping_add(81);
    total = total.wrapping_add(82);
    total
}

// ------------------------------------------------ allowed lints stay quiet

/// Doubles `n` without `#[must_use]`.
// quiet: clippy::must_use_candidate
pub fn double(n: u32) -> u32 {
    n.saturating_mul(2)
}

/// Repeats the module name in a type name.
#[derive(Debug)]
// quiet: clippy::module_name_repetitions
pub struct ViolationsReport;

/// A module named after its parent.
// quiet: clippy::module_inception
pub mod violations {
    /// Lives in `violations::violations`.
    pub fn nested() {}
}

/// A shape to match on.
#[derive(Debug, Clone, Copy)]
pub enum Shape {
    /// Round.
    Round,
    /// Square.
    Square,
}

/// Imports an enum's variants with a glob inside the function that matches.
pub fn corners(shape: Shape) -> u32 {
    // quiet: clippy::enum_glob_use
    use Shape::*;
    match shape {
        Round => 0,
        Square => 4,
    }
}
