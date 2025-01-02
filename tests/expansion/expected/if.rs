#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use oxiplate_derive::Oxiplate;
#[oxiplate_inline = "
{%- if do_this -%}
    This then {{ action }} :D
{%- endif %}"]
struct Data {
    do_this: bool,
    action: &'static str,
}
impl std::fmt::Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.do_this {
            f.write_fmt(
                format_args!("{0}{1}{2}{3}{4}", "This then", " ", self.action, " ", ":D"),
            )?;
        }
        Ok(())
    }
}
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "test_if"]
#[doc(hidden)]
pub const test_if: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_if"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests\\if.rs",
        start_line: 14usize,
        start_col: 4usize,
        end_line: 14usize,
        end_col: 11usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_if())),
};
fn test_if() {
    let data = Data {
        do_this: true,
        action: "do something",
    };
    match (
        &::alloc::__export::must_use({
            let res = ::alloc::fmt::format(format_args!("{0}", data));
            res
        }),
        &"This then do something :D",
    ) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&test_if])
}
