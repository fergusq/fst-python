# KFST binary format

KFST uses a simple binary format for storing transducers.
Its purpose is to reduce the size of the transducer files and to speed up loading them.

## Format

The basic structure of a file is as follows:

|Number of bytes|Data type|Description|
|---|---|---|
|17|-|Header|
|*variable*|sequence of null-terminated UTF-8 strings|Symbols|
|*variable*|LZMA encoded data|States and final states|

Each file starts with a header, which contains the following fields:

|Number of bytes|Data type|Description|
|---|---|---|
|4|unsigned int|Magic number (The string `KFST` encoded in ascii)|
|2|unsigned short|Version number (Currently 0)|
|2|unsigned short|Number of symbols|
|4|unsigned int|Number of states|
|4|unsigned int|Number of final states|
|1|bool|0 = not weighted, 1 = weighted|

After the header, there is a list of symbols.
Each symbol is encoded as a null-terminated UTF-8 string.

After the symbols, there is a block of LZMA encoded data that first contains the states and then the final states.

Each state is encoded as follows:

|Number of bytes|Data type|Description|
|---|---|---|
|4|unsigned int|Previous state|
|4|unsigned int|Next state|
|2|unsigned short|Input symbol|
|2|unsigned short|Output symbol|
|8|double|Weight (absent if the transducer is not weighted)|

The final states are encoded as follows:

|Number of bytes|Data type|Description|
|---|---|---|
|4|unsigned int|State|
|8|double|Weight (absent if the transducer is not weighted)|