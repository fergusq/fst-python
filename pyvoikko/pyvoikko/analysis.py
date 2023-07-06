import re
from typing import NamedTuple

from .constants import MAPS, FIELD_NAMES

class VoikkoAnalysis(NamedTuple):
    """
    Represents a single analysis of a word.
    Refer to the Voikko documentation for more information about the fields.

    Attributes:
        FORM: surface form (This is not included in the native Voikko output)
        BASEFORM: base form
        CLASS: part of speech
        FSTOUTPUT: the original output from the FST
        INFO_FLAGS: additional information about the analysis (This is not included in the native Voikko output)
        MOOD: mood of verb
        NEGATIVE: negation of verb (either `"true"`, `"false"` or `"both"`)
        PERSON: person of verb
        TENSE: tense of verb
        SIJAMUOTO: case of noun
        NUMBER: number of noun
        FOCUS: particles like -kin, -kaan of noun
        POSSESSIVE: possessive suffix of noun
        COMPARISON: comparison form of adjective
        PARTICIPLE: if the form is a participle of a verb, it will be analysed as an adjective, and this field will be set to indicate that the word is in fact a participle
        COMPOUND_PARTS: if the word is a compound, this field will contain a list of `VoikkoAnalysis` objects representing the individual parts of the compound
    """

    FORM: str
    BASEFORM: str
    CLASS: str
    FSTOUTPUT: str
    INFO_FLAGS: list[str]
    
    # Verb attributes
    MOOD: str | None = None
    NEGATIVE: str | None = None
    PERSON: str | None = None
    TENSE: str | None = None

    # Noun attributes
    SIJAMUOTO: str | None = None
    NUMBER: str | None = None
    FOCUS: str | None = None
    POSSESSIVE: str | None = None

    # Adjective attributes
    COMPARISON: str | None = None
    PARTICIPLE: str | None = None

    COMPOUND_PARTS: list["VoikkoAnalysis"] | None = None

    @staticmethod
    def from_voikko_analysis(analysis: str):
        """
        Parses a single analysis from the output of the Voikko FST.

        Args:
            analysis: the analysis to parse

        Returns:
            a `VoikkoAnalysis` object
        """
        # Split to parts and tags
        parts = re.split(r"(\[[^\]]+\])", analysis)
        parts = [part for part in parts if part != ""]

        # Split to compound parts at [Bc] tags
        compound_parts = []
        current_part = []
        for part in parts:
            if part == "[Bc]":
                compound_parts.append(current_part)
                current_part = []
            else:
                current_part.append(part)
        
        compound_parts.append(current_part)

        # Return the analysis
        analyses = [VoikkoAnalysis._from_unitary_voikko_analysis(part) for part in compound_parts]

        if len(analyses) == 1:
            return analyses[0]
        
        return analyses[-1]._replace(
            COMPOUND_PARTS=analyses,
            FSTOUTPUT=analysis,
            FORM = "".join([analysis.FORM for analysis in analyses]),
            BASEFORM = "".join([analysis.BASEFORM + ("-" if analysis.FORM.endswith("-") else "") for analysis in analyses]),
        )

    @staticmethod
    def _from_unitary_voikko_analysis(parts: list[str]):
        in_Xp = False
        in_Xr = False

        baseform = ""
        surface_form = ""

        xp_string = ""
        x_string = ""

        properties = {}
        flags = []

        for part in parts:
            if part == "[Xp]" or part == "[Xj]":
                in_Xp = True
                baseform += x_string
                surface_form += x_string
                x_string = ""
                xp_string = ""
            
            elif part == "[Xr]" or part == "[Xs]":
                in_Xr = True
            
            elif part == "[X]":
                in_Xp = in_Xr = False
            
            elif is_tag(part):
                name, arg = parse_tag(part)
                if name == "I":
                    flags.append(name+arg)

                elif name == "D":
                    flags.append(name+arg)

                elif name in FIELD_NAMES:
                    properties[FIELD_NAMES[name]] = MAPS[name].get(arg, arg)
            
            elif in_Xr:
                pass

            elif in_Xp:
                xp_string += part
            
            else:
                x_string += part
        
        baseform += xp_string
        surface_form += x_string

        if not baseform:
            baseform = surface_form[:-1] if surface_form.endswith("-") else surface_form
        
        if "De" in flags and surface_form.endswith("is-"):
            baseform = surface_form[:-1]
        
        is_lower = "Dg" in flags
        is_capitalize = "De" in flags

        properties["BASEFORM"] = baseform.lower() if is_lower else baseform.capitalize() if is_capitalize else baseform
        properties["FORM"] = surface_form
        properties["FSTOUTPUT"] = "".join(parts)
        properties["INFO_FLAGS"] = flags

        return VoikkoAnalysis(**properties)


def is_tag(part: str):
    return part.startswith("[")


def parse_tag(tag: str):
    tag = tag[1:-1]
    return tag[0], tag[1:]