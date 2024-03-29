# mdbook tools

A repository of tools that can be used with [mdbook](https://rust-lang.github.io/mdBook/), particularily for collections of fiction works.

All of the code in this repository is licensed under the terms of the MIT License or the Apache 2.0 License, at your option.

## add-copyright

add-copyright is a preprocessor that can be used. It conditionally replaces the string `!{#copyright}` in markdown files with the content of a file named `COPYRIGHT-STUB.md` (or specified in the config) relative to the book source directory.
Relative links are repointed to keep the target consistent with the stub file.

The program produces a binary called `mdbook-add-copyright`, so you can enable the preprocessor simply by adding `[preprocessor.add-copyright]` to your `book.toml`.

### Config

In addition to the config keys for all preprocessors, the `add-copyright` preprocessor accepts the following config keys.
All keys are optional

```toml
[preprocessor.add-copyright]
# The file to use for the replacement, relative to the source directory of the book
# defaults to `COPYRIGHT-STUB.md`
copyright-stub = "path/to/copyright-stub.md"
# Specifies the set of files the replacement is performed on
# If omitted, includes all chapters specified in the book's `SUMMARY.md`
include = ["file1.md", "file2.md"]
# Specifies the set of files the replacement is not performed on
# defaults to an empty list
exclude = ["file3.md"]

# Additional config per-renderer.
# Inherits the global configuration
[preprocessor.add-copyright.<renderer>]
# Specifies the set of files the replacement is performed on when preprocessing the input for <renderer>
# If omitted, includes all chapters specified in the book's `SUMMARY.md`
include = ["file1.md", "file2.md"]
# Specifies the set of files the replacement is not performed on when preprocessing the input for <renderer>
# defaults to an empty list
exclude = ["file3.md"]
```

#### File Sets

The `include` and `exclude` key (and the renderer-specific variants) allows for granular control over the files replacement is performed on - only the files specified in `include` are included, and none of the files specified by `exclude` are included (regardless of whether they appear in an `include`). To affect all markdown files in the book (except for explicitly excluded ones), you can omit the `include` key. 

The files are relative to the src directory - currently the values must be direct paths, globbing is not yet supported (but may be in the future). 

When render-specific file sets are specified, the `include` set is an intersection between the global config and the per-renderer config, and the `exclude` set is a union - the renderer specific config only removes files that are included for all renderers.

Note that when a file is not included, the preprocessor is still run on that file. The string `!{#copyright}` is instead removed from the string. 
This is useful for appending copyright info to the bottom of each chapter when using a renderer that displays chapters in separate pages (such as the `html` backend), but omitting it when multiple chapters may be appended together (such as the `epub-fancy` backend).

## epub-fancy

epub-fancy is an mdbook backend that emits epub files. This backend supports substantial configuration to properly support a number of epub features.

Like the `add-copyright` preprocessor, this installs a binary called `mdbook-epub-fancy`, so the backend can be enabled directly by adding `[output.epub-fancy]` to your `book.toml`.

### Config

The config fields supported by the `epub-fancy` backend are:

```toml
[output.epub-fancy]
# Sets which outputs are provided,
# Valid values `full` (generate a single epub file for the entire book), `part` (generate an epub file for each Header separated part), or `chapter` (generate individual epub files for each chapter - NOT YET IMPLEMENTED)
# Multiple options can be specified as follows, each type of output is generated
# output=["full", "part"]
output="<type>"

# Not Yet Implemented:
# Save intermediate files generated by the backend in the output directory (under a directory named by the output file id when multiple types are specified or the type is not `full`)
# The current implementation writes all files directory to the zip container  and does not currently support generating the relevant temp files
# The option is parsed (validated as a `bool`) but ignored by the renderer. It may be implemented in the future. 
# save-temps = false

# When generating `part` or `chapter` outputs, always include these files in each output.
always-include = ["list/of/files.md"]

# Allows specifying the unique identifier (dc:identifier) for the epub package documents in each output file
[output.epub-fancy.file-ids]
# Allows specifying the unique identifier when generating the `full` output.
# Exactly one of `uuid`, `oid`, or `isbn` may be specified (otherwise the table must be omitted)
# The default is a suitably unique `uuid` (current implementation generates a v7 id based on the current time)
[output.epub-fancy.file-ids.full]
# Specifies a unique identifier that is a Universally Unique Identifier (https://datatracker.ietf.org/doc/html/rfc4122)
uuid = "<uuid>"
# Specifies a unique identifier that is an Object Identifier
oid = "<oid>
# Specifies a unique identifier that is an ISBN (ISBN 10 or ISBN 13)
isbn = "<isbn>

# Allows specifying the unique identifier for specific part or chapter outputs
# Exactly one of `uuid`, `oid`, or `isbn` may be specified (otherwise the table must be omitted)
# The default is a suitably unique `uuid` (current implementation generates a v7 id based on the current time)
# The key is derived from the part or chapter title
[output.epub-fancy.file-ids.'<output-id>']
# Specifies a unique identifier that is a Universally Unique Identifier (https://datatracker.ietf.org/doc/html/rfc4122)
uuid = "<uuid>"
# Specifies a unique identifier that is an Object Identifier
oid = "<oid>
# Specifies a unique identifier that is an ISBN (ISBN 10 or ISBN 13)
isbn = "<isbn>


# Advanced configuration option - changes the name of the epub output files
[output.epub-fancy.output-files]
# Specifies the name of the file for the `full` output
# Defaults to a name derived from the book title
full = "full.epub"
# Specifies the name of the file for the given identified output
# Used for part and chapter outputs
# The key for a given part or chapter is derived from the part/chapter title
# The default is the key with `.epub` appended
'<output-id>'="<output-id>.epub"
```

### Output Ids

When using the `file-ids` or `output-files` configuration tables, outputs corresponding to the `part` or `chapter` output type use a computed output id.

These types are computed as follows:
* For part outputs, if the part heading in the `SUMMARY.md` file contains a cmark extension heading id specifier (`.id` inside `{}` after the text of the header), the exact id is used. Note that whitespace ends the id specification.
* For chapter outputs, the check is made on the first h1 heading of the chapters file, provided that no input other than blank lines appear before that heading
* Otherwise, the whole title specified in `SUMMARY.md` is converted as follows:
  1. Leading and trailing whitespace is trimmed.
  2. Nonwhitespace Characters other than letters, digits, and the characters `_` and `-` are removed.
  3. Each nonempty sequence of whitespace is replaced with a single `-`.
  4. Each upper case letter is replaced with the corresponding lower case letter.


Note that currently mdbook does not have support for heading extension support in `SUMMARY.md` specifically, so on non-aware renderers (including the builtin `html` backend), heading extension specifiers will be displayed verbatim. 