# mathematica-notebook-filter

`mathematica-notebook-filter` is a program written in
[Rust](https://www.rust-lang.org/) that parses Mathematica notebook files and
strips them of superfluous information so that they can be committed into
version control systems more easily.  Instructions to integrate this program
into version control systems can be found [below](#integration).

[![Travis](https://img.shields.io/travis/JP-Ellis/mathematica-notebook-filter/master.svg)](https://travis-ci.org/JP-Ellis/mathematica-notebook-filter)
[![Codecov](https://img.shields.io/codecov/c/github/JP-Ellis/mathematica-notebook-filter/master.svg)](https://codecov.io/gh/JP-Ellis/mathematica-notebook-filter)

Licensed under [GPLv3](https://www.gnu.org/licenses/gpl-3.0.html).

*This program has not been rigorously tested.  It works for me on all my
Notebooks, but there may still be some situations which have not been accounted
for.  If you use this program, please let me know (both good and bad).*

## Introduction

Version control systems (such as [git](https://git-scm.com/) and
[mercurial](https://www.mercurial-scm.org/) among many others) provide a
fantastic way to keep track of changes to files in such a way that multiple
people can collaborate on them without accidentally overwriting other people's
changes.  Version control systems primarily keep track of source code and if two
people change the same file, it is possible to compare the two files
side-by-side so that the changes can be merged.

Although binary files (such as compiled outputs, images, PDFs, ...) can be
included in a version control system too, it is generally not possible or
meaningful to compare two sets of changes to one binary file.  As a result,
binary files are quite opaque to version control systems and it is inadvisable
to store binary files in a version control system if they will be changed
frequently.

This is specifically an issue for Mathematica notebooks as they store both
inputs and outputs in the same file.  A quite typical example of this is the
simple input:

```mathematica
Plot[Sin[x] / x, {x, -4 Pi, 4 Pi}]
```

which, when plotted, is stored in the Notebook file as:

```mathematica
GraphicsBox[{{{{}, {},
  TagBox[
       {RGBColor[0.368417, 0.506779, 0.709798], AbsoluteThickness[1.6],
        Opacity[1.], LineBox[CompressedData["<omitted>"]],
        LineBox[CompressedData["<omitted>"]]},
       Annotation[#,
        "Charting`Private`Tag$5185#1"]& ], {}}, {{}, {}, {}}}, {}, {}},
   AspectRatio->NCache[GoldenRatio^(-1), 0.6180339887498948],
   Axes->{True, True},
   AxesLabel->{None, None},
   AxesOrigin->{0, 0},
   DisplayFunction->Identity,
   Frame->{{False, False}, {False, False}},
   FrameLabel->{{None, None}, {None, None}},
   FrameTicks->{{Automatic,
      Charting`ScaledFrameTicks[{Identity, Identity}]}, {Automatic,
      Charting`ScaledFrameTicks[{Identity, Identity}]}},
   GridLines->{None, None},
   GridLinesStyle->Directive[
     GrayLevel[0.5, 0.4]],
   ImagePadding->All,
   Method->{
    "DefaultBoundaryStyle" -> Automatic, "DefaultMeshStyle" ->
     AbsolutePointSize[6], "ScalingFunctions" -> None,
     "CoordinatesToolOptions" -> {"DisplayFunction" -> ({
         (Identity[#]& )[
          Part[#, 1]],
         (Identity[#]& )[
          Part[#, 2]]}& ), "CopiedValueFunction" -> ({
         (Identity[#]& )[
          Part[#, 1]],
         (Identity[#]& )[
          Part[#, 2]]}& )}},
   PlotRange->
    NCache[{{(-4) Pi, 4 Pi}, {-0.21723358083481298`,
      0.9999892952885239}}, {{-12.566370614359172`,
     12.566370614359172`}, {-0.21723358083481298`, 0.9999892952885239}}],
   PlotRangeClipping->True,
   PlotRangePadding->{{
      Scaled[0.02],
      Scaled[0.02]}, {
      Scaled[0.05],
      Scaled[0.05]}},
   Ticks->{Automatic, Automatic}]
```

Note that the above snippet was significantly abbreviated as the compressed
base-64 encoded data is an additional 300 lines or so.

For the version control system, this large output is extremely cumbersome as a
small change in the input (such as replacing `Sin[x]` with `Sin[2 x]`) will
produce a 300+ line diff.  The purpose of `mathematica-notebook-filter` is
specifically to avoid such large diffs and try and make them much more
meaningful.  It does so by parsing the Mathematica notebook file format and
removing all the output cells and metadata.  The program is implemented in
[Rust](https://www.rust-lang.org/) and distributed on
[crates.io](https://crates.io/crates/mathematica-notebook-filter).

Having said that, it should be noted that Mathematica unfortunately does not
store the input in a very simple form as it not only stores the plain
Mathematica expression, but also stores formatting information.  As a concrete
example, an input cell with the above plot function will be stored in the
Notebook file as:

```mathematica
Cell[BoxData[
 RowBox[{"Plot", "[",
  RowBox[{
   FractionBox[
    RowBox[{"Sin", "[", "x", "]"}], "x"], ",",
   RowBox[{"{",
    RowBox[{"x", ",",
     RowBox[{
      RowBox[{"-", "4"}], "Pi"}], ",",
     RowBox[{"4", "Pi"}]}], "}"}]}], "]"}]], "Input"]
```

The change of `Sin[x]` to `Sin[2 x]` results in the cell now being stored as:

```mathematica
Cell[BoxData[
 RowBox[{"Plot", "[",
  RowBox[{
   FractionBox[
    RowBox[{"Sin", "[",
     RowBox[{"2", "x"}], "]"}], "x"], ",",
   RowBox[{"{",
    RowBox[{"x", ",",
     RowBox[{
      RowBox[{"-", "4"}], "Pi"}], ",",
     RowBox[{"4", "Pi"}]}], "}"}]}], "]"}]], "Input"]
```

This program, at least at this stage, will *not* strip the extra formatting
information.  If you wish to avoid the above, then you should save your
notebooks as scripts files (with extension `.wl` or `.m`).

## Usage Notes

`mathematica-notebook-filter` parses Mathematica notebook files (usually stored
with the extension `.nb`) and strips all generated outputs and other metadata.
The program reads from standard input and outputs to standard output.
Currently, the program offers has no options.

This program is not designed to be used on its own and should be integrated with
version control systems (see [below](#Integration) for instructions).  If you
wish to run it manually, a simple call would be:

```sh
cat my_notebook.nb | mathematica-notebook-filter > my_notebook_cleaned.nb
```

This program does *not* parse the Wolfram language in general and is specific to
*full* Mathematica notebooks; thus it makes some fairly strong assumptions about
the functions that will be found and their order.  It only parses a single
Notebook at a time and will stop after the end of the first Notebook.  If an
error is encountered during the parsing, `mathematica-notebook-filter` will exit
with a non-zero code and the output will be left incomplete.

It also should be re-iterated that the best way to commit Mathematica code to a
version control system is to save the code in script files (`.wl` or `.m`).
When doing so, Mathematica save the file in a very simple format (essentially a
plain text file), without the superfluous formatting information and without
outputs.  This unfortunately has the disadvantage that the Notebook interface is
not available.

Also note that Mathematica notebooks allow you to copy-paste graphics (such as
generated plots) and use them as inputs.  If you do so, the version control
system will be forced to include the full plot in the diff, thereby defeating
the point of `mathematica-notebook-filter`.  An alternative to copy-pasting
outputs is to store the output into a variable, or use `%` (and `%%`, `%%%`,
...) to refer to the previous output (though make sure to only use `%` within
the one cell and not across cells as `%` refers to the last generated output,
not the previous output in the Notebook order).

## Installation

This program is written in [Rust](https://www.rust-lang.org/).  Probably the
easiest way to install Rust is to use the [rustup.rs](https://www.rustup.rs/)
script.  Once set up, it should simply be a matter of running

```sh
cargo install mathematica-notebook-filter
```

This will download, compile, and install `mathematica-notebook-filter` in your
Cargo home direction (`~/.cargo` by default on Linux).  Assuming you have
correctly set up your PATH variable (which rustup.rs should have done
automatically), then you can execute the program by typing
`mathematica-notebook-filter`.

## Integration

### Git

It is possible to set *attributes* based on pattern globs.  In this instance, we
want to make sure that all `*.nb` files are processed by this filter before
being committed.  To globally set the attribute, add to `~/.gitattributes`:

```text
*.nb    filter=dropoutput_nb
```

and to your `~/.gitconfig`:

```text
[filter "dropoutput_nb"]
    clean = mathematica-notebook-filter
    smudge = cat
```

### Other

Pull requests to add instructions for other version control system are welcome.


Disclaimer
==========

The Wolfram Research organization unfortunately does not appear to offer any
specification to their language or their file formats.  As a result, this filter
was entirely developed by inspecting outputs generated by Mathematica.
Specifically, this was developed using Mathematica 11.1 and thus there is no
guarantee that this filter will work with past or future version of the Notebook
file format.

If you find a bug, please feel free to open an issue though please provide
enough information to reproduce the bug or a minimal example of a Notebook file
that causes the issue.

Contributing
============

Pull requests to improve compatibility with other versions (or to fix bugs) are
very welcome.  If you find a bug, please feel free to open an issue and make
sure to provide enough information to reproduce the bug or a minimal example of
a Notebook file that causes the issue.
