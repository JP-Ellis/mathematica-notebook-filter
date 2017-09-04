use std::io;

use super::cell::parse_cell_list;
use super::utilities::load_rest_of_function;

/// Parse the `Notebook[]` function.
///
/// Background
/// ==========
///
/// This is the main function that defines the Notebook's contents.  It's
/// general format is:
///
/// ```mathematica
/// Notebook[{
///   cell1,
///   cell2,
///   ...
/// }]
/// ```
///
/// In addition, optional arguments can be specified such as:
///
/// ```mathematica
/// WindowSize->{808, 911},
/// WindowMargins->{{4, Automatic}, {Automatic, 30}},
/// FrontEndVersion->"11.1 for Linux x86 (64-bit) (April 18, 2017)",
/// ```
pub fn parse_notebook<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    parse_notebook_start(input, output)
        .and(parse_cell_list(input, output))
        .and(parse_notebook_end(input, output))
}

/// Parse the start of the `Notebook[]` function.
///
/// The input will be consumed until the `Notebook` function is reached, and it
/// will output everything from the `Notebook` function until the opening brace
/// (but not including the opening brace).
fn parse_notebook_start<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    let notebook_bytes = &b"Notebook["[..];

    let pos = {
        let buf = input.fill_buf()?;
        buf.windows(notebook_bytes.len()).position(
            |w| w == notebook_bytes,
        )
    };

    match pos {
        Some(pos) => {
            input.consume(pos);

            let brace_pos = {
                let buf = input.fill_buf()?;
                buf.iter().position(|&c| c == b'{')
            };

            match brace_pos {
                Some(brace_pos) => {
                    {
                        let buf = input.fill_buf()?;
                        output.write_all(&buf[..brace_pos])?;
                    }
                    input.consume(brace_pos);
                    Ok(())
                }
                None => {
                    Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "EOF reached before finding start of the list of cells within `Notebook[]`.",
                    ))
                }

            }
        }
        None => {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "EOF reached before finding the `Notebook[]` function.",
            ))
        }
    }
}

/// Parse the end of the `Notebook[]` function.
///
/// The input should be at the end of the list of cells, just after the closing
/// brace.
fn parse_notebook_end<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    // It appears that all of the optional arguments at the end of the Notebook
    // should probably be removed.  We just write the final closing bracket and
    // the end of notebook content specification.
    let (s, _) = load_rest_of_function(input)?;
    match s.first() {
        Some(&b',') | Some(&b']') => output.write_all(b"]\n(* End of Notebook Content *)\n"),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected to be right after an argument when parsing the end of `Notebook[]`.",
        )),
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn notebook_start() {
        let mut output = Vec::new();
        let mut input = &b"(* Beginning of Notebook Content *)\nNotebook[{\n\nCell["[..];

        assert!(super::parse_notebook_start(&mut input, &mut output).is_ok());
        assert_eq!(input, &b"{\n\nCell["[..]);
        assert_eq!(output.as_slice(), &b"Notebook["[..]);
    }

    #[test]
    fn notebook_end() {
        let mut output = Vec::new();
        let mut input = &br#",
WindowSize->{808, 911},
WindowMargins->{{4, Automatic}, {Automatic, 30}},
FrontEndVersion->"11.1 for Linux x86 (64-bit) (April 18, 2017)",
StyleDefinitions->FrontEnd`FileName[{$RootDirectory, "home", "josh", "src", 
   "Mathematica"}, "Stylesheet.nb", CharacterEncoding -> "UTF-8"]
]
(* End of Notebook Content *)

(* Internal cache information *)
(*CellTagsOutline
CellTagsIndex->{}
*)
(*CellTagsIndex
CellTagsIndex->{}
*)"#
            [..];
        assert!(super::parse_notebook_end(&mut input, &mut output).is_ok());
        assert_eq!(
            input,
            &br#"
(* End of Notebook Content *)

(* Internal cache information *)
(*CellTagsOutline
CellTagsIndex->{}
*)
(*CellTagsIndex
CellTagsIndex->{}
*)"#
                [..]
        );
        assert_eq!(
            output.as_slice(),
            &b"]\n(* End of Notebook Content *)\n"[..]
        );
    }

}
