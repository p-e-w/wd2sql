# `wd2sql`

`wd2sql` is a tool that transforms a
[Wikidata](https://www.wikidata.org/wiki/Wikidata:Main_Page) JSON dump
into a fully indexed SQLite database that is 90% smaller than the original
dump, yet contains most of its information. The resulting database enables
high-performance queries to be executed on commodity hardware without the
need to install and configure specialized triplestore software. Most
programming languages have excellent support for SQLite, and lots of relevant
tools exist. I believe this to be by far the easiest option for working with
a local copy of Wikidata that is currently available.

`wd2sql` is *much* faster than most other dump processing tools. In fact,
it can usually process JSON data as fast as `bzip2` can decompress it.
It uses native code, SIMD-accelerated JSON parsing, an optimized allocator,
batched transactions, prepared statements, and other SQLite optimizations
to achieve that performance. On a 2015 consumer laptop, it processes a full
dump of Wikidata (1.5 Terabytes) in less than 12 hours, using only around
10 Megabytes of RAM.

`wd2sql` is **not**

* a general-purpose triplestore. It makes assumptions about the structure
  of the dump that are specific to Wikidata, and will fail when run on other
  semantic databases.
* a complete replacement for traditional datastores such as Wikibase.
  In particular, the SQLite database currently does not contain sitelinks,
  aliases, qualifiers, references, non-English labels and descriptions,
  and a few other pieces of information that are present in the dumps.
* in any way affiliated with, or endorsed by, the Wikidata project and/or
  the Wikimedia Foundation.


## License

Copyright &copy; 2022  Philipp Emanuel Weidmann (<pew@worldwidemann.com>)

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.

**By contributing to this project, you agree to release your
contributions under the same license.**
