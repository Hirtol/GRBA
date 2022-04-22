/// Creates a [`syn::Error`] with the format message and infers the
/// [`Span`](`proc_macro2::Span`) using [`Spanned`](`syn::spanned::Spanned`).
///
/// # Parameters
///
/// - The first argument must be a type that implements [`syn::spanned::Spanned`].
/// - The second argument is a format string.
/// - The rest are format string arguments.
macro_rules! format_err {
    ( $spanned:expr, $($msg:tt)* ) => {{
        ::syn::Error::new(
            <_ as ::syn::spanned::Spanned>::span(&$spanned),
            format_args!($($msg)*)
        )
    }}
}

pub trait CombineError {
    /// Combines `self` with the given `another` error and returns back combined `self`.
    fn into_combine(self, another: syn::Error) -> Self;
}

impl CombineError for syn::Error {
    fn into_combine(mut self, another: syn::Error) -> Self {
        self.combine(another);
        self
    }
}
