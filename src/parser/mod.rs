use std::io;

mod header;
mod notebook;
mod cell;
mod cell_group_data;
mod utilities;
mod whitespace;

use self::header::parse_header;
use self::notebook::parse_notebook;
use self::whitespace::WhitespaceCleaner;

/// Parse the input and write to output the stripped Mathematica notebook.
pub fn parse_input<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    debug!("Parsing input.");

    let mut output = WhitespaceCleaner::new(output);

    parse_header(input, &mut output).and(parse_notebook(input, &mut output))
}
