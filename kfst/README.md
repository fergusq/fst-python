# KFST

KFST is a finite state transducer library written in pure Python. It supports reading transducers from [AT&T tabular format](https://github.com/hfst/hfst/blob/master/doc/transducer-representations-formats.rst) files. In addition to standard features, it also supports flag diacritics.

The intended use case is to use KFST in environments in which native libraries like HFST cannot be installed.

## Installation

KFST is available on PyPI and can be installed with pip:

```
pip install kfst
```

## Usage

In this example, we assume that you have a transducer stored in the file `my-transducer.att`.
You can easily create such file using the HFST toolkit:

```sh
hfst-fst2txt -f att my-transducer.hfst > my-transducer.att
```

### Reading transducers

Transducers can be read from AT&T tabular format files using the `read_att` function:

```python
from kfst import FST

fst = FST.read_att_file("my-transducer.att")
```

### Using the transducer

To run the transducer, use the `lookup` method, which returns a list of tuples of the form `(output, weight)` sorted by weight:

```python
fst.lookup("foo")
```

## License

KFST is licensed under the GNU LGPL version 3 or later. See the LICENSE file for details.
