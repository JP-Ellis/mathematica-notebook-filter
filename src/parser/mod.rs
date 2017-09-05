use std::io;

mod header;
mod notebook;
mod cell;
mod cell_group_data;
mod utilities;

use self::header::parse_header;
use self::notebook::parse_notebook;

pub fn parse_input<I, O>(input: &mut I, output: &mut O) -> Result<(), io::Error>
where
    I: io::BufRead,
    O: io::Write,
{
    parse_header(input, output).and(parse_notebook(input, output))
}
