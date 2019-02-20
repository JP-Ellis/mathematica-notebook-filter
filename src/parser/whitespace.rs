use std::io;
use std::io::BufRead;

/// A simple intermediate buffer that removes trailing whitespaces on each line
/// that it is given before passing it on to the underlying output.
///
/// This assumes that the contents of the file can be separated into lines and
/// thus assumes that it is valid UTF8.
pub struct WhitespaceCleaner<O> {
    line_buffer: String,
    output: O,
}

impl<O> WhitespaceCleaner<O>
where
    O: io::Write,
{
    /// Create a new instance of `WhitespaceCleaner` with the underlying output.
    pub fn new(output: O) -> Self {
        WhitespaceCleaner {
            line_buffer: String::with_capacity(1024 * 8),
            output,
        }
    }
}

impl<O> io::Write for WhitespaceCleaner<O>
where
    O: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut bytes_read = 0;
        while !(&buf[bytes_read..]).is_empty() {
            bytes_read += (&buf[bytes_read..]).read_line(&mut self.line_buffer)?;
            // read_line will read up to the next newline, or the end of the
            // buffer.  We only want to strip trailing whitespaces if it *is*
            // the end of the line and not the end of the buffer.
            if self.line_buffer.as_bytes().last() == Some(&0xA) {
                {
                    let s = self.line_buffer.trim_end();
                    self.output.write_all(s.as_bytes())?;
                    self.output.write_all(&[0xA])?;
                }
                self.line_buffer.clear();
            }
        }

        Ok(bytes_read)
    }

    /// Flush the current content of the buffer and force it to be given to the
    /// underlying output.
    ///
    /// # Warning
    ///
    /// Unless the internal buffer is was empty, there is no guarantee that the
    /// output will have trailing whitespaces removed.
    fn flush(&mut self) -> io::Result<()> {
        if self.line_buffer.is_empty() {
            Ok(())
        } else {
            self.output.write_all(self.line_buffer.as_bytes())?;
            self.line_buffer.clear();
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use super::WhitespaceCleaner;

    #[test]
    fn write() {
        // First with nicely terminated lines
        let mut output: Vec<u8> = Vec::new();
        let input = &br#"
 the quick   
brown fox    
    jump     
over   	     
   the       
lazy dog.    
"#[..];

        {
            let mut wc = WhitespaceCleaner::new(&mut output);
            wc.write_all(input).is_ok();
            wc.flush().is_ok();
        }
        assert_eq!(
            &String::from_utf8(output).unwrap(),
            r#"
 the quick
brown fox
    jump
over
   the
lazy dog.
"#
        );

        // Try a slightly messier output
        let mut output: Vec<u8> = Vec::new();
        let input = "abc  \n123 \n456";

        {
            let mut wc = WhitespaceCleaner::new(&mut output);
            wc.write_all(input.as_bytes()).is_ok();
            wc.flush().is_ok();
        }
        assert_eq!(&String::from_utf8(output).unwrap(), "abc\n123\n456");
    }
}
