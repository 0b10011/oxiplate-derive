#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use std::io::Write;
use std::{error::Error, fs::self, path::Path, process::{Command, Output}};
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "clippy"]
#[doc(hidden)]
pub const clippy: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("clippy"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests\\clippy.rs",
        start_line: 10usize,
        start_col: 4usize,
        end_line: 10usize,
        end_col: 10usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(clippy())),
};
fn clippy() -> Result<(), Box<dyn Error>> {
    let expected_destination = Path::new("tests/clippy/expected/");
    let actual_destination = Path::new("tests/clippy/actual/");
    let mut mismatched = 0;
    for entry in fs::read_dir("tests/clippy/src/bin")? {
        let entry = entry?;
        let expected_output_path = expected_destination
            .join(
                ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(
                        format_args!("{0}.stderr", entry.file_name().to_string_lossy()),
                    );
                    res
                }),
            );
        let actual_output_path = actual_destination
            .join(
                ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(
                        format_args!("{0}.stderr", entry.file_name().to_string_lossy()),
                    );
                    res
                }),
            );
        if !entry.file_type()?.is_file()
            || entry.path().to_str() == Some("tests\\clippy.rs")
        {
            continue;
        }
        let test_name_path = entry.path().with_extension("");
        let test_name = test_name_path
            .file_name()
            .ok_or("failed to read filename")?
            .to_string_lossy();
        let Output { status, stdout, stderr } = Command::new("cargo")
            .args([
                "clippy",
                "--bin",
                &test_name,
                "--manifest-path",
                "tests/clippy/Cargo.toml",
            ])
            .output()?;
        if status.success() {
            Err(
                ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(
                        format_args!(
                            "Clippy passed for \'{0}\' when it shouldn\'t have. STDOUT: {1}",
                            test_name, String::from_utf8_lossy(& stdout)
                        ),
                    );
                    res
                }),
            )?;
        }
        let actual_expansion = String::from_utf8_lossy(&stderr);
        if let Ok(expected_expansion) = std::fs::read_to_string(&expected_output_path) {
            let same = actual_expansion.lines().eq(expected_expansion.lines());
            if same {
                std::io::stdout()
                    .write_fmt(format_args!("expansion of {0} ... ok\n", test_name))?;
            } else {
                std::io::stdout()
                    .write_fmt(
                        format_args!("expansion of {0} ... mismatched\n", test_name),
                    )?;
                std::fs::write(actual_output_path.clone(), actual_expansion.as_bytes())?;
                mismatched += 1;
                let Output { status: _, stdout, stderr: _ } = Command::new("git")
                    .args([
                        "diff",
                        "--color",
                        "--no-index",
                        "--",
                        expected_output_path.to_str().unwrap(),
                        &actual_output_path.to_string_lossy(),
                    ])
                    .output()?;
                std::io::stdout()
                    .write_fmt(
                        format_args!("\n{0}\n", String::from_utf8_lossy(& stdout)),
                    )?;
            }
        } else {
            std::fs::write(actual_output_path, actual_expansion.as_bytes())?;
            mismatched += 1;
            std::io::stdout()
                .write_fmt(
                    format_args!(
                        "expansion of {0} ... tests/clippy/expected/{1}.rs.stderr is missing\n",
                        test_name, test_name
                    ),
                )?;
        }
    }
    if mismatched > 0 {
        Err("One or more expansions test results were mismatched or missing")?;
    }
    Ok(())
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&clippy])
}
