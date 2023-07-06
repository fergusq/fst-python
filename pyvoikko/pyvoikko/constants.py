
CLASS_MAP = {
	"n": "nimisana",
	"l": "laatusana",
	"nl": "nimisana_laatusana",
	"h": "huudahdussana",
	"ee": "etunimi",
	"es": "sukunimi",
	"ep": "paikannimi",
	"em": "nimi",
	"t": "teonsana",
	"a": "lyhenne",
	"s": "seikkasana",
	"u": "lukusana",
	"ur": "lukusana",
	"r": "asemosana",
	"c": "sidesana",
	"d": "suhdesana",
	"k": "kieltosana",
	"p": "etuliite",
}

SIJAMUOTO_MAP = {	
	"n": "nimento",
	"g": "omanto",
	"p": "osanto",
	"es": "olento",
	"tr": "tulento",
	"ine": "sisaolento",
	"ela": "sisaeronto",
	"ill": "sisatulento",
	"ade": "ulkoolento",
	"abl": "ulkoeronto",
	"all": "ulkotulento",
	"ab": "vajanto",
	"ko": "seuranto",
	"in": "keinonto",
	"sti": "kerrontosti",
	"ak": "kohdanto",
}

COMPARISON_MAP = {	
	"c": "comparative",
	"s": "superlative",
}

MOOD_MAP = {
	"n1": "A-infinitive",
	"n2": "E-infinitive",
	"n3": "MA-infinitive",
	"n4": "MINEN-infinitive",
	"n5": "MAINEN-infinitive",
	"t": "indicative",
	"e": "conditional",
	"k": "imperative",
	"m": "potential",
}

NUMBER_MAP = {	
	"y": "singular",
	"m": "plural",
}

PERSON_MAP = {
	"1": "1",
	"2": "2",
	"3": "3",
	"4": "4",
}

TENSE_MAP = {
	"p": "present_simple",
	"i": "past_imperfective",
}

FOCUS_MAP = {
	"kin": "kin",
	"kaan": "kaan",
}

POSSESSIVE_MAP = {
	"1y": "1s",
	"2y": "2s",
	"1m": "1p",
	"2m": "2p",
	"3": "3",
}

NEGATIVE_MAP = {
	"t": "true",
	"f": "false",
	"b": "both",
}

PARTICIPLE_MAP = {
	"v": "present_active",
	"a": "present_passive",
	"u": "past_active",
	"t": "past_passive",
	"m": "agent",
	"e": "negation",
}

MAPS = {
    "L": CLASS_MAP,
    "S": SIJAMUOTO_MAP,
    "C": COMPARISON_MAP,
    "T": MOOD_MAP,
    "N": NUMBER_MAP,
    "P": PERSON_MAP,
    "A": TENSE_MAP,
    "F": FOCUS_MAP,
    "O": POSSESSIVE_MAP,
    "E": NEGATIVE_MAP,
    "R": PARTICIPLE_MAP,
}

FIELD_NAMES = {
    "L": "CLASS",
    "S": "SIJAMUOTO",
    "C": "COMPARISON",
    "T": "MOOD",
    "N": "NUMBER",
    "P": "PERSON",
    "A": "TENSE",
    "F": "FOCUS",
    "O": "POSSESSIVE",
    "E": "NEGATIVE",
    "R": "PARTICIPLE",
}