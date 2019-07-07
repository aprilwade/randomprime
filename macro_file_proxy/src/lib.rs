use proc_macro_hack::proc_macro_hack;

#[proc_macro_hack]
pub use macro_file_proxy_proc_macro::macro_file_proxy;

pub use macro_file_proxy_proc_macro::macro_file_proxy_item;

#[cfg(test)]
mod test
{
    use super::*;
    macro_rules! int_test_macro {
        ($(line $i:expr,)+) => { const _CONST: &[u32] =&[$($i,)+]; };
    }
    macro_file_proxy_item!("test_input.txt", int_test_macro, ,);
}
#[test]
fn test()
{
    macro_rules! int_test_macro {
        ($(line $i:expr,)+) => {&[$($i,)+]};
    }
    assert_eq!(
        macro_file_proxy!("test_input.txt", int_test_macro, ,),
        &[1, 2, 3, 4],
    );

    macro_rules! str_test_macro {
        ($($id:ident $i:expr;)+) => {&[$(stringify!($id $i),)+]};
    }
    assert_eq!(
        macro_file_proxy!("test_input.txt", str_test_macro, ;),
        &["line 1", "line 2", "line 3", "line 4"],
    );
}
