}  pHP4        �                                                                                                           } .h1 WordStar to Markdown

This is a small utility that converts WordStar (.ws) into Markdown (.md) files.

.h2 Installation

.lm 1
% cargo install ws2markdown
.lm

.h2 Usage

.lm 1
% ws2markdown input.ws [output.md]
.lm

If you don't provide an input file, a file dialog will appear.
If you don't provide an output file, the resulting Markdown will be printed to stdout.

.h2 Examples

This README is a proof that it works.
.. Comment lines are ignored, by the way.  