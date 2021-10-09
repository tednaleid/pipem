# pipem - pipe merge

A basic command line app that lets you pipe it a delimited list of records/fields (just like awk) and it will fill in placeholders in a template string.

ex: 
```
seq 3 | pipem 'https://example.com/{1}?key=value'
https://example.com/1?key=value
https://example.com/2?key=value
https://example.com/3?key=value
```

in addition to singular fields (`{2}` emits field 2) it also supports field ranges (`{2,5}` will emit fields 2 through 5) :

```
cat << EOF | pipem 'first: {1} both: {1,2}'
apple one
banana two
carrot three
EOF
first: apple both: apple one
first: banana both: banana two
first: carrot both: carrot three
```

and unbounded ranges with a starting field (`{2,}` emits field 2 through however many fields there are on the line):

```
cat << EOF | pipem 'ragged: {2,}'
apple one
banana two more values here
carrot three even some more values
EOF
ragged: one
ragged: two more values here
ragged: three even some more values
```

It's simpler to use and about 2x as fast as the equivalent awk command


# Installation

Currently requires [rustup/cargo](https://rustup.rs/), there will eventually be releases.

```
cargo install --path .
```
