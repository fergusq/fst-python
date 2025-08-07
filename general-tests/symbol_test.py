from kfst.symbols import *

class Ext(Symbol):
    def __init__(self, text):
        self.payload = text
    
    def get_symbol(self) -> str:
        return self.payload
    
    def is_epsilon(self) -> bool:
        return False
    
    def is_unknown(self) -> bool:
        return False
    
    def __eq__(self, other) -> bool:
        if isinstance(other, Ext):
            return self.payload == other.payload
        else:
            return False
    
    def __lt__(self, other) -> bool:
        if isinstance(other, Ext):
            return self.payload < other.payload
        else:
            return True

    def __gt__(self, other) -> bool:
        if isinstance(other, Ext):
            return self.payload > other.payload
        else:
            return False
    
    def __repr__(self) -> str:
        return f"Ext{self.get_symbol()}"

print("Check that external symbol generally works")

k = [Ext("Teelikamenten"), Ext("Tenten"), Ext("Enten"), StringSymbol("Tenten")]
assert sorted(k) == [Ext("Enten"), Ext("Teelikamenten"), Ext("Tenten"), StringSymbol(string='Tenten', unknown=False)], sorted(k)

print("Check default values of direct symbol initialisation")

assert StringSymbol("X") == StringSymbol("X", False)
