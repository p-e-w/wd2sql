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


## Installation

Install [Rust](https://www.rust-lang.org/) 1.61 or later, then run

```
cargo install wd2sql
```

This will compile `wd2sql` for your native CPU architecture, which is crucial
for performance.

Note that while `wd2sql` *should* work on all platforms, I have only tested
it on Linux.


## Usage

```
wd2sql <JSON_FILE> <SQLITE_FILE>
```

Use `-` as `<JSON_FILE>` to read from standard input instead of from a file.
This makes it possible to build a pipeline that processes JSON data as it is
being decompressed, without having to decompress the full dump to disk:

```
bzcat latest-all.json.bz2 | wd2sql - output.db
```


## Database structure

### IDs

Wikidata IDs consist of a type prefix (`Q`/`P`/`L`) plus an integer.
`wd2sql` encodes both of these as a single 32-bit integer (64-bit for
form and sense IDs):

* **Entity IDs** are simply represented as the integer part of their ID.
  For example, `Q42` becomes `42`.
* **Property IDs** are represented as the integer part of their ID,
  plus 1 billion. For example, `P31` becomes `1000000031`.
* **Lexeme IDs** are represented as the integer part of their ID,
  plus 2 billion. For example, `L234` becomes `2000000234`.
* **Form IDs** are represented as the encoded ID of their associated lexeme
  (see above), plus 100 billion times their integer form ID.
  For example, `L99-F2` becomes `202000000099`.
* **Sense IDs** are represented as the encoded ID of their associated lexeme
  (see above), plus 100 billion times their integer sense ID, plus 10 billion.
  For example, `L99-S1` becomes `112000000099`.

This encoding is simple and compact, and can be easily applied both
automatically by algorithms, and manually by humans.

### Tables

In all tables, the `id` column contains the Wikidata ID of the subject entity,
encoded as described above. The following tables are generated:

* `meta`, which contains the English `label` and `description` for each entity,
  or `NULL` if the entity doesn't have an English label or description.
* `string`, `entity`, `coordinates`, `quantity`, and `time`, which contain
  the values of claims associated with each entity. The table in which an
  individual claim value is stored corresponds to the property's
  [value type](https://www.wikidata.org/wiki/Special:ListDatatypes),
  and the property is identified by the `property_id` column.
* `none` and `unknown`, which contain `id`/`property_id` pairs identifying
  claims whose value is "no value" and "unknown value", respectively.

### Example: Finding red fruits

First, we need to obtain the IDs of the relevant entities:

```
sqlite> SELECT * FROM meta WHERE label = 'red';

id         label  description
---------  -----  ----------------------------------------------------------
17126729   red    eye color
101063203  red    2018 video game by Bart Bonte
3142       red    color
29713895   red    genetic element in the species Drosophila melanogaster
29714596   red    protein-coding gene in the species Drosophila melanogaster
```

From these results, we can see that the entity we are interested in
(the color red) has ID `3142`. Repeating this procedure reveals that
"fruit (food)" has ID `3314483`, and the properties "subclass of" and
"color (of subject)" have IDs `1000000279` and `1000000462`, respectively.

Both "red" and "fruit" are entities, so claims about them can be found
in the table `entity`. We can now easily construct a query that returns
the desired information:

```
sqlite> SELECT * FROM meta WHERE
   ...> id IN (SELECT id FROM entity WHERE property_id = 1000000462 AND entity_id = 3142)
   ...> AND id IN (SELECT id FROM entity WHERE property_id = 1000000279 AND entity_id = 3314483);

id        label        description
--------  -----------  --------------------------------------------------------------------------------------------------------
89        apple        fruit of the apple tree
196       cherry       fruit of the cherry tree
503       banana       elongated, edible fruit produced by several kinds of large herbaceous flowering plants in the genus Musa
2746643   fig          edible fruit of Ficus carica
13202263  peach        fruit, use Q13189 for the species
13222088  pomegranate  fruit of Punica granatum
```

All of these queries have sub-second execution times, and the results
are identical to those that can be obtained with the SPARQL query

```
SELECT ?item ?itemLabel
WHERE
{
  ?item wdt:P462 wd:Q3142.
  ?item wdt:P279 wd:Q3314483.
  SERVICE wikibase:label { bd:serviceParam wikibase:language "en". }
}
```

from the Wikidata Query Service.


## Acknowledgments

`wd2sql` depends on the crates
[`lazy_static`](https://github.com/rust-lang-nursery/lazy-static.rs),
[`clap`](https://github.com/clap-rs/clap),
[`rusqlite`](https://github.com/rusqlite/rusqlite),
[`simd-json`](https://github.com/simd-lite/simd-json),
[`wikidata`](https://github.com/Smittyvb/wikidata),
[`chrono`](https://github.com/chronotope/chrono),
[`humansize`](https://github.com/LeopoldArkham/humansize),
[`humantime`](https://github.com/tailhook/humantime),
and [`jemallocator`](https://github.com/tikv/jemallocator).

Without the efforts of the countless people who built Wikidata and its
contents, `wd2sql` would be useless. It's truly impossible to praise
this amazing open data project enough.


## Related projects

[import-wikidata-dump-to-couchdb](https://github.com/maxlath/import-wikidata-dump-to-couchdb)
is a tool that transfers Wikidata dumps to a CouchDB document database.

[Knowledge Graph Toolkit](https://github.com/usc-isi-i2/kgtk) (KGTK)
is a (much more comprehensive) system for working with semantic data,
which includes functionality for importing Wikidata dumps.

[dumpster-dive](https://github.com/spencermountain/dumpster-dive)
is a conceptually similar tool that parses *Wikipedia* dumps and
stores the result in a MongoDB database.


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
