use std::io;

use super::cell::parse_cell_list;
use super::utilities::{load_rest_of_function, read_consume_output};

/// Parse the `Notebook[]` function.
///
/// This function does not require the input to be at the start of the notebook,
/// and instead will consume everything up to it.  It does expect that the
/// `Notebook[]` function is coming up soon.
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
    debug!("Parsing Notebook function.");

    let notebook_bytes = b"Notebook[";
    let pos = {
        let buf = input.fill_buf()?;
        buf.windows(notebook_bytes.len())
            .position(|w| w == notebook_bytes)
    };
    match pos {
        Some(pos) => {
            input.consume(pos);
            parse_notebook_start(input, output)
                .and(parse_cell_list(input, output))
                .and(parse_notebook_end(input, output))
        }
        None => Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Unable to locate the `Notebook[]` function.",
        )),
    }
}

/// Parse the start of the `Notebook[]` function.
///
/// This function assumes that the input is at the start of the Notebook
/// function.
fn parse_notebook_start<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    debug!("Parsing start of Notebook function.");

    let pos = {
        let buf = input.fill_buf()?;
        buf.iter().position(|&c| c == b'{')
    };
    match pos {
        Some(pos) => read_consume_output(input, output, pos),
        None => Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "EOF reached before finding start of the list of cells within `Notebook[]`.",
        )),
    }
}

/// Parse the end of the `Notebook[]` function.
///
/// The input should be at the end of the list of cells, just after the closing
/// brace.
///
/// Since `Notebook` does not appear to store any optional information that
/// ought to be kept, this function simply consumes everything up to the closing
/// bracket of the function.
///
/// Finally, it adds the information `(* End of Notebook Content *)` to the end
/// of the output.
fn parse_notebook_end<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    debug!("Parsing end of Notebook function.");

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
        // Valid
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &b"Notebook[{\n\nCell["[..];
        assert!(super::parse_notebook_start(&mut input, &mut output).is_ok());
        assert_eq!(input, b"{\n\nCell[");
        assert_eq!(&output, b"Notebook[");

        // Invalid
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &b"Notebook[\n\nCell["[..];
        assert!(super::parse_notebook_start(&mut input, &mut output).is_err());

        let mut output = Vec::new();
        let mut input = &b"Cell[\n\nCell["[..];
        assert!(super::parse_notebook_start(&mut input, &mut output).is_err());
    }

    #[test]
    fn notebook_end() {
        // Valid
        ////////////////////////////////////////
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
*)"#[..];
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
*)"#[..]
        );
        assert_eq!(
            output.as_slice(),
            &b"]\n(* End of Notebook Content *)\n"[..]
        );

        // Invalid
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &br"WindowSize->{808, 911}"[..];
        assert!(super::parse_notebook_end(&mut input, &mut output).is_err());
    }

    #[test]
    fn notebook() {
        // Valid
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &br#"
(* Beginning of Notebook Content *)
Notebook[{
  Cell[1],
  Cell[2]
},
WindowSize->{1272, 1534},
WindowMargins->{{4, Automatic}, {Automatic, 30}},
FrontEndVersion->"11.1 for Linux x86 (64-bit) (April 18, 2017)",
StyleDefinitions->FrontEnd`FileName[{$RootDirectory, "home", "josh", "src", 
   "Mathematica"}, "Stylesheet.nb", CharacterEncoding -> "UTF-8"]
]
(* End of Notebook Content *)
"#[..];
        assert!(super::parse_notebook(&mut input, &mut output).is_ok());
        assert_eq!(input, &b"\n(* End of Notebook Content *)\n"[..]);
        assert_eq!(
            output,
            &br#"Notebook[{
  Cell[1],
  Cell[2]
}]
(* End of Notebook Content *)
"#[..]
        );

        // Invalid
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &br#"CellGroupData[{
  Cell[1],
  Cell[2]
}]"#[..];
        assert!(super::parse_notebook(&mut input, &mut output).is_err());
    }

}
