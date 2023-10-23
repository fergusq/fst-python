# This file is part of KFST.
#
# (c) 2023 Iikka Hauhio <iikka.hauhio@helsinki.fi>
#
# KFST is free software: you can redistribute it and/or modify it under the
# terms of the GNU Lesser General Public License as published by the Free
# Software Foundation, either version 3 of the License, or (at your option) any
# later version.
#
# KFST is distributed in the hope that it will be useful, but WITHOUT ANY
# WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
# FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public License for more
# details.
#
# You should have received a copy of the GNU Lesser General Public License
# along with KFST. If not, see <https://www.gnu.org/licenses/>.

from enum import Enum
from typing import Hashable, NamedTuple, Literal, Protocol


class Symbol(Hashable, Protocol):
    """
    Defines the interface that all symbol classes should implement.
    """
    
    def is_epsilon(self) -> bool:
        """
        Returns True if this symbol should be regarded as an epsilon symbol.
        """
        ...
    
    def is_unknown(self) -> bool:
        """
        Returns True if this symbol should be regarded as an unknown symbol.
        """
        ...
    
    def get_symbol(self) -> str:
        """
        Returns the string representation of the symbol.
        """
        ...


class StringSymbol(NamedTuple):
    """
    Represents a symbol in the input alphabet.
    """
    string: str
    unknown: bool = False

    def is_epsilon(self):
        return False
    
    def is_unknown(self):
        return self.unknown
    
    def get_symbol(self):
        return self.string


class FlagDiacriticSymbol(NamedTuple):
    """
    Represents a flag diacretic.
    """
    flag_type: Literal["U", "R", "D", "C", "P", "N"]
    key: str
    value: str | None

    def is_epsilon(self):
        return True
    
    def is_unknown(self):
        return False
    
    def get_symbol(self):
        if self.value is None:
            return f"@{self.flag_type}.{self.key}@"
        
        else:
            return f"@{self.flag_type}.{self.key}.{self.value}@"

    @staticmethod
    def is_flag_diacritic(symbol: str) -> bool:
        """
        Returns True if the given string symbol is a flag diacretic.
        """
        return len(symbol) > 4 and symbol[0] == "@" and symbol[-1] == "@" and symbol[1] in "PNDRCU" and symbol[2] == "."

    @staticmethod
    def from_symbol_string(symbol: str) -> "FlagDiacriticSymbol":
        """
        Parses flag diacretic symbols.
        
        * Two params: `@<flag type>.<flag key>.<flag value>@`
        * One param: `@<flag type>.<flag key>@`
        """
        assert FlagDiacriticSymbol.is_flag_diacritic(symbol)
        flag = symbol[1]
        di = symbol.rindex(".")
        key = symbol[3:di] if di > 3 else symbol[3:-1]
        value = symbol[di+1:-1] if di > 3 else None
        return FlagDiacriticSymbol(flag, key, value)  # type: ignore


class SpecialSymbol(Enum):
    """
    Enum for miscellaneous special symbols.
    """
    EPSILON = "@_EPSILON_SYMBOL_@"
    IDENTITY = "@_IDENTITY_SYMBOL_@"
    UNKNOWN = "@_UNKNOWN_SYMBOL_@"

    def is_epsilon(self):
        return self == SpecialSymbol.EPSILON
    
    def is_unknown(self):
        return False
    
    def get_symbol(self):
        return self.value
    
    @staticmethod
    def is_special_symbol(symbol: str) -> bool:
        """
        Returns True if the given string symbol is a special symbol.
        """
        return symbol in {"@0@", "@_EPSILON_SYMBOL_@", "@_IDENTITY_SYMBOL_@", "@_UNKNOWN_SYMBOL_@"}

    @staticmethod
    def from_symbol_string(symbol: str) -> "SpecialSymbol":
        """
        Parses a special symbol string to a SpecialSymbol value.
        """
        assert SpecialSymbol.is_special_symbol(symbol)
        if symbol == "@0@" or symbol == "@_EPSILON_SYMBOL_@":
            return SpecialSymbol.EPSILON

        elif symbol == "@_IDENTITY_SYMBOL_@":
            return SpecialSymbol.IDENTITY

        elif symbol == "@_UNKNOWN_SYMBOL_@":
            return SpecialSymbol.UNKNOWN
        
        assert False


def from_symbol_string(symbol: str) -> Symbol:
    """
    Parses a symbol string into a Symbol object.
    """
    if FlagDiacriticSymbol.is_flag_diacritic(symbol):
        return FlagDiacriticSymbol.from_symbol_string(symbol)
    
    elif SpecialSymbol.is_special_symbol(symbol):
        return SpecialSymbol.from_symbol_string(symbol)
    
    else:
        return StringSymbol(symbol)
