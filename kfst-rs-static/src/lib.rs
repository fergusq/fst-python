extern crate proc_macro;
use std::collections::HashMap;

use kfst_rs::{EmbeddedFST, RefContainer};
use proc_macro::TokenStream;
use syn::{LitStr, parse_macro_input};
/*
#[proc_macro]
pub fn embed_fst(path: TokenStream) -> TokenStream {
    let input = parse_macro_input!(path as LitStr);
    let fst = kfst_rs::FST::from_kfst_file(input.value(), false).unwrap();
    let embedded_fst = kfst_rs::EmbeddedFST::from_fst(fst);

    // Actual conversion into token streams

    // We should get the same thing back and these are small
    let _final_states: proc_macro2::TokenStream = format!("{:?}", embedded_fst._final_states).parse().unwrap();
    let _symbols: proc_macro2::TokenStream = format!("{:?}", embedded_fst._symbols).parse().unwrap();

    // True pain in the ass

    let mut _rules = String::new();
    _rules.push('[');
    for x in embedded_fst._rules {
        _rules.push('(');
        _rules.extend(format!("{:?}", x.0).chars());
        _rules.extend(",&[".chars());
        for y in x.1 {
            _rules.push('(');
            _rules.extend(format!("{:?}", y.0).chars());
            _rules.extend(",&[".chars());
            for z in y.1 {
                _rules.extend(format!("{:?},", z).chars());
            }
            _rules.extend("]),".chars());
        }
        _rules.extend("]),".chars());
    }
    _rules.push(']');
    let _rules: proc_macro2::TokenStream = _rules.parse().unwrap();
    
    TokenStream::from(quote::quote!(
        {
        static u: &'static [(u64, &[(&str, &[(u64, &str, f64)])])] = &#_rules;
        static k: kfst_rs::EmbeddedFST<kfst_rs::RefContainer> = kfst_rs::EmbeddedFST {
            _final_states: &#_final_states,
            _rules: u,
            _symbols: &#_symbols
         };
        &k
        }
    ))
    /*let w: &[(&str, &[(usize, &str, f64)])] = &[("a",&[(1, "b", 0.0)][..]),][..];
    w[0].1[0];
    let k: &dyn std::ops::Index<usize, Output=(&str, &[(usize, &str, f64)])> = &[];
    let k = format!("EmbeddedFST {{\n  _rules: {},\n  _final_states: {},\n  _symbols: {}\n}}", _rules, format!("{:?}", embedded_fst._final_states), format!("{:?}", embedded_fst._symbols));*/
    //format!("{:?}", k).parse().unwrap()
}*/


#[proc_macro]
pub fn embed_fst(path: TokenStream) -> TokenStream {
    let input = parse_macro_input!(path as LitStr);
    let fst = kfst_rs::FST::from_kfst_file(input.value(), false).unwrap();
    let embedded_fst = kfst_rs::EmbeddedFST::from_fst(fst);

    // Actual conversion into token streams

    // We should get the same thing back and these are small
    let _final_states: proc_macro2::TokenStream = format!("{:?}", embedded_fst._final_states).parse().unwrap();
    let _symbols: proc_macro2::TokenStream = format!("{:?}", embedded_fst._symbols).parse().unwrap();
    let mut symbol_lookup: HashMap<String, u16> = HashMap::new();

    for (idx, symbol) in embedded_fst._symbols.iter().enumerate() {
        symbol_lookup.insert(symbol.to_string(), idx.try_into().unwrap());
    }

    // True pain in the ass


    let start_states: proc_macro2::TokenStream = format!("{:?}", embedded_fst.source_states).parse().unwrap();
    let target_states: proc_macro2::TokenStream = format!("{:?}", embedded_fst.target_states).parse().unwrap();
    let top_symbols: proc_macro2::TokenStream = format!("{:?}", embedded_fst.top_symbols).parse().unwrap();
    let bottom_symbols: proc_macro2::TokenStream = format!("{:?}", embedded_fst.bottom_symbols).parse().unwrap();
    let weights: proc_macro2::TokenStream = format!("{:?}", embedded_fst.weights).parse().unwrap();
    let weightset: proc_macro2::TokenStream = format!("{:?}", embedded_fst.weightset).parse().unwrap();
    
    /*TokenStream::from(quote::quote!(
        {
        static k: &'static (&[u64], &[u64], &[u16], &[u16], &[f64]) = &(&#start_states, &#target_states, &#top_symbols, &#bottom_symbols, &#weights);
        &k
        }
    ))*/
    TokenStream::from(quote::quote!(
        {
        static r: &'static (&[u32], &[u32], &[u16], &[u16], &[u8]) = &(&#start_states, &#target_states, &#top_symbols, &#bottom_symbols, &#weights);
        static k: kfst_rs::EmbeddedFST<kfst_rs::RefContainer> = kfst_rs::EmbeddedFST {
            _final_states: &#_final_states,
            source_states: &r.0,
            target_states: &r.1,
            top_symbols: &r.2,
            bottom_symbols: &r.3,
            weights: &r.4,
            _symbols: &#_symbols,
            weightset: &#weightset
         };
        &k
        }
    ))
}


#[proc_macro]
pub fn embed_pykko(path: TokenStream) -> TokenStream {
    let input = parse_macro_input!(path as LitStr);
    let fst = kfst_rs::FST::from_kfst_file(input.value(), false).unwrap();
    let embedded_fst = kfst_rs::EmbeddedFST::from_fst(fst);

    // Actual conversion into token streams

    // We should get the same thing back and these are small
    let _final_states: proc_macro2::TokenStream = format!("{:?}", embedded_fst._final_states.iter().map(|x| x.0).collect::<Vec<_>>()).parse().unwrap();
    let _symbols: proc_macro2::TokenStream = format!("{:?}", embedded_fst._symbols).parse().unwrap();
    let mut symbol_lookup: HashMap<String, u16> = HashMap::new();

    for (idx, symbol) in embedded_fst._symbols.iter().enumerate() {
        symbol_lookup.insert(symbol.to_string(), idx.try_into().unwrap());
    }

    // True pain in the ass


    let start_states: proc_macro2::TokenStream = format!("{:?}", embedded_fst.source_states).parse().unwrap();
    let target_states: proc_macro2::TokenStream = format!("{:?}", embedded_fst.target_states).parse().unwrap();
    let top_symbols: proc_macro2::TokenStream = format!("{:?}", embedded_fst.top_symbols).parse().unwrap();
    let bottom_symbols: proc_macro2::TokenStream = format!("{:?}", embedded_fst.bottom_symbols).parse().unwrap();
    
    /*TokenStream::from(quote::quote!(
        {
        static k: &'static (&[u64], &[u64], &[u16], &[u16], &[f64]) = &(&#start_states, &#target_states, &#top_symbols, &#bottom_symbols, &#weights);
        &k
        }
    ))*/

    TokenStream::from(quote::quote!(
        {
        static r: &'static (&[u32], &[u32], &[u16], &[u16]) = &(&#start_states, &#target_states, &#top_symbols, &#bottom_symbols);
        static k: Pykko = Pykko {
            final_states: &#_final_states,
            source_states: &r.0,
            target_states: &r.1,
            top_symbols: &r.2,
            bottom_symbols: &r.3,
            symbols: &#_symbols,
         };
        &k
        }
    ))
}