use std::io;

/// Parse the header of the Notebook file.
///
/// The header consists of everything up to the beginning of the Mathematica
/// code, which in this case is the function `Notebook[]`.
pub fn parse_header<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    debug!("Parsing notebook header.");

    parse_content_type(input, output).and(parse_beginning_notebook(input, output))
}

/// Parse the input until the content type specification is reached, and pass
/// that on to the output.
///
/// ```mathematica
/// (* Content-type: application/vnd.wolfram.mathematica *)
/// ```
///
/// The consume everything up to (and including) the closing parenthesis.  The
/// output include an extra new line after the content type.
fn parse_content_type<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    debug!("Parsing notebook content-type.");

    let content_bytes = b"(* Content-type: ";

    let pos = {
        let buf = input.fill_buf()?;
        buf.windows(content_bytes.len()).position(
            |w| w == content_bytes,
        )
    };

    match pos {
        Some(pos) => {
            input.consume(pos);
            let mut out = Vec::new();
            input.read_until(b')', &mut out)?;
            output.write_all(out.as_slice())?;
            output.write_all(b"\n")?;
            Ok(())
        }
        None => {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unable to locate the content type specification near the start of the file.",
            ))
        }
    }
}

/// Parse the input until the beginning of the notebook specification is
/// reached, and pass that on to the output.
///
/// ```mathematica
/// (* Beginning of Notebook Content *)
/// ```
///
/// The consume everything up to (and including) the closing parenthesis.  The
/// output includes an extra new line before and after the beginning of notebook
/// specification.
fn parse_beginning_notebook<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    debug!("Parsing beginning of notebook specification.");

    let beginning_bytes = &b"(* Beginning of Notebook Content *)"[..];

    let pos = {
        let buf = input.fill_buf()?;
        buf.windows(beginning_bytes.len()).position(
            |w| w == beginning_bytes,
        )
    };

    match pos {
        Some(pos) => {
            input.consume(pos);
            let mut out = Vec::new();
            input.read_until(b')', &mut out)?;
            output.write_all(b"\n")?;
            output.write_all(out.as_slice())?;
            output.write_all(b"\n")?;
            Ok(())
        }
        None => {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unable to locate the beginning of Notebook content specification.",
            ))
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn content_type() {
        // Extra before input
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &b"ABC(* Content-type: application/vnd.wolfram.mathematica *)\n"[..];
        assert!(super::parse_content_type(&mut input, &mut output).is_ok());
        assert_eq!(input, b"\n");
        assert_eq!(
            output,
            &b"(* Content-type: application/vnd.wolfram.mathematica *)\n"[..]
        );

        // Longer valid input
        ////////////////////////////////////////
        let mut output = Vec::new();
        let mut input = &b"(* Content-type: application/vnd.wolfram.mathematica *)\n\
                                 \n\
                                 (*** Wolfram Notebook File ***)\n\
                                 (* http://www.wolfram.com/nb *)"
            [..];
        assert!(super::parse_content_type(&mut input, &mut output).is_ok());
        assert_eq!(
            input,
            &b"\n\n(*** Wolfram Notebook File ***)\n(* http://www.wolfram.com/nb *)"[..]
        );
        assert_eq!(
            output,
            &b"(* Content-type: application/vnd.wolfram.mathematica *)\n"[..]
        );
    }

    #[test]
    fn beginning_notebook() {
        let mut output = Vec::new();
        let mut input = &b"(* Other stuff *)\n\
                                  \n\
                                  (* Beginning of Notebook Content *)\n\
                                  Notebook[{\n\
                                  \n"
            [..];
        assert!(super::parse_beginning_notebook(&mut input, &mut output).is_ok());
        assert_eq!(input, b"\nNotebook[{\n\n");
        assert_eq!(output, &b"\n(* Beginning of Notebook Content *)\n"[..]);
    }
}
