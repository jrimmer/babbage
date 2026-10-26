use std::collections::HashMap;
use std::sync::OnceLock;

use crate::error::Error;
use crate::number::Number;

const BASES: usize = 8;

/// Exponents of the base dimensions: length, mass, time, temperature, current, amount, data (bit), angle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Dim([i8; BASES]);

impl Dim {
    pub const NONE: Dim = Dim([0; BASES]);

    fn parse(spec: &str) -> Dim {
        let mut d = [0i8; BASES];
        for part in spec.split_whitespace() {
            let (letter, exp) = part.split_at(1);
            let slot = "LMTKINBA"
                .find(letter)
                .unwrap_or_else(|| panic!("bad dimension letter in {spec:?}"));
            d[slot] = if exp.is_empty() {
                1
            } else {
                exp.parse().expect("dimension exponent")
            };
        }
        Dim(d)
    }

    pub fn is_none(self) -> bool {
        self == Dim::NONE
    }

    fn scaled(self, by: i8) -> Dim {
        let mut d = self.0;
        for e in &mut d {
            *e = e.saturating_mul(by);
        }
        Dim(d)
    }

    fn plus(self, other: Dim) -> Dim {
        let mut d = self.0;
        for (e, o) in d.iter_mut().zip(other.0) {
            *e = e.saturating_add(o);
        }
        Dim(d)
    }
}

pub type UnitId = u16;

struct UnitDef {
    symbol: &'static str,
    dim: Dim,
    factor: Number,
    /// Kelvin at zero of this unit, for scales that do not start at absolute zero.
    offset: Number,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UnitExpr(Vec<(UnitId, i8)>);

pub struct Registry {
    defs: Vec<UnitDef>,
    by_name: HashMap<String, UnitExpr>,
}

/// Where each line comes from: `symbol | names | dimension | exact factor to SI base`, then optional
/// `+prefixes` lines that add SI-prefixed copies. `@names = unit expression` defines a compound alias.
/// Factors are exact (`*`, `/`, `^` over decimals). Where the reference lists a rounded value, the
/// exact standard definition is used and the difference is recorded in docs/units.md.
const TABLE: &str = r#"
# time
s | s,sec,secs,second,seconds | T | 1
+n,µ,m
min | min,mins,minute,minutes | T | 60
h | h,hr,hrs,hour,hours | T | 3600
day | day,days | T | 86400
wk | wk,wks,week,weeks | T | 604800
mo | mo,month,months | T | 30.436875*86400
yr | yr,yrs,year,years | T | 365.2425*86400
# length
m | m,meter,meters,metre,metres | L | 1
+n,µ,m,c,d,k
in | inch,inches | L | 0.0254
ft | ft,foot,feet | L | 0.3048
yd | yd,yds,yard,yards | L | 0.9144
mi | mi,mile,miles | L | 1609.344
fur | fur,furlong,furlongs | L | 201.168
ftm | ftm,fathom,fathoms | L | 1.8288
NM | NM,nmi,nautical mile,nautical miles | L | 1852
ly | ly,light year,light years | L | 9460730472580800
px | px,pixel,pixels | L | 0.0254/96
pt | pt,point,points | L | 0.0254/72
# area and volume that are not powers of a length unit
ac | ac,acre,acres | L2 | 4046.8564224
ha | ha,hectare,hectares | L2 | 10000
are | are,ares | L2 | 100
L | L,l,liter,liters,litre,litres | L3 | 0.001
+m,c,d,k,M
mcup | metric cup,metric cups | L3 | 0.00025
gal | gal,gallon,gallons | L3 | 0.003785411784
qt | qt,quart,quarts | L3 | 0.003785411784/4
pint | pint,pints | L3 | 0.003785411784/8
cup | cup,cups | L3 | 0.003785411784/16
floz | fl oz,floz,fluid ounce,fluid ounces | L3 | 0.003785411784/128
tbsp | tbsp,tablespoon,tablespoons | L3 | 0.003785411784/256
tsp | tsp,teaspoon,teaspoons | L3 | 0.003785411784/768
bsh | bsh,bushel,bushels | L3 | 0.03523907016688
af | af,acre foot,acre feet,acre-foot | L3 | 4046.8564224*0.3048
# mass
g | g,gram,grams,gramme,grammes | M | 0.001
+n,µ,m,k
mcg | mcg | M | 0.000000001
lb | lb,lbs,pound,pounds | M | 0.45359237
oz | oz,ounce,ounces | M | 0.45359237/16
st | st,stone,stones | M | 0.45359237*14
ton | ton,tons,short ton,short tons | M | 0.45359237*2000
t | t,tonne,tonnes,metric ton,metric tons | M | 1000
ozt | oz t,ozt,troy ounce,troy ounces | M | 0.0311034768
ct | ct,carat,carats | M | 0.0002
# angle (degrees; radians need an irrational factor and are not defined yet)
deg | °,deg,degree,degrees | A | 1
arcmin | ′,arcmin,arcminute,arcminutes | A | 1/60
arcsec | ″,arcsec,arcsecond,arcseconds | A | 1/3600
rev | rev,revolution,revolutions | A | 360
# temperature (affine: the fifth column is the offset to kelvin)
K | K,kelvin | K | 1
°C | °C,ºC,degC,celsius,C | K | 1 | 273.15
°F | °F,ºF,degF,fahrenheit,F | K | 5/9 | 459.67*5/9
# frequency
Hz | Hz,hertz | T-1 | 1
+n,µ,m,k,M,G,T
# energy, power, force, pressure
J | J,joule,joules | M L2 T-2 | 1
+k,M,G
Wh | Wh,watt hour,watt hours | M L2 T-2 | 3600
+k,M,G,T
cal | cal,calorie,calories | M L2 T-2 | 4.184
kcal | kcal,kCal,Cal,kilocalorie,kilocalories | M L2 T-2 | 4184
W | W,watt,watts | M L2 T-3 | 1
+m,k,M,G,T
hp | hp,horsepower | M L2 T-3 | 746
N | N,newton,newtons | M L T-2 | 1
+n,µ,m,k,M,G
lbf | lbf,pound-force,pounds-force | M L T-2 | 0.45359237*9.80665
pdl | pdl,poundal,poundals | M L T-2 | 0.45359237*0.3048
ozf | ozf | M L T-2 | 0.45359237*9.80665/16
Pa | Pa,pascal,pascals | M L-1 T-2 | 1
+h,k,M,G
bar | bar,bars | M L-1 T-2 | 100000
mbar | mbar | M L-1 T-2 | 100
atm | atm,atmosphere,atmospheres | M L-1 T-2 | 101325
psi | psi | M L-1 T-2 | 0.45359237*9.80665/(0.0254^2)
ksi | ksi | M L-1 T-2 | 1000*0.45359237*9.80665/(0.0254^2)
Mpsi | Mpsi | M L-1 T-2 | 1000000*0.45359237*9.80665/(0.0254^2)
# amount
mol | mol,mole,moles | N | 1
+m,µ,n
# electrical
V | V,volt,volts | M L2 T-3 I-1 | 1
+m,µ,k,M
KV | KV | M L2 T-3 I-1 | 1000
A | A,amp,amps,ampere,amperes | I | 1
+m,µ,k,M
ohm | Ω,ohm,ohms | M L2 T-3 I-2 | 1
+m,µ,k,M
coulomb | coulomb,coulombs | T I | 1
Ah | Ah,ampere hour,ampere hours | T I | 3600
+m,µ,k,M
farad | farad,farads | M-1 L-2 T4 I2 | 1
+m,µ,k,M
henry | henry,henrys,henries | M L2 T-2 I-2 | 1
+m,µ,k,M
siemens | siemens | M-1 L-2 T3 I2 | 1
+m,µ,k,M
Wb | Wb,weber,webers | M L2 T-2 I-1 | 1
+m,µ,k,M
tesla | tesla,teslas | M T-2 I-1 | 1
+m,µ,k,M
# data (bit is the base)
bit | bit,bits | B | 1
B | B,byte,bytes | B | 8
+k,M,G,T,P,E,Z,Y
KiB | KiB,kibibyte,kibibytes | B | 8*1024
MiB | MiB,mebibyte,mebibytes | B | 8*1024^2
GiB | GiB,gibibyte,gibibytes | B | 8*1024^3
TiB | TiB,tebibyte,tebibytes | B | 8*1024^4
PiB | PiB,pebibyte,pebibytes | B | 8*1024^5
EiB | EiB,exbibyte,exbibytes | B | 8*1024^6
ZiB | ZiB,zebibyte,zebibytes | B | 8*1024^7
YiB | YiB,yobibyte,yobibytes | B | 8*1024^8
kb | kb,kilobit,kilobits | B | 1000
Mb | Mb,megabit,megabits | B | 1000^2
Gb | Gb,gigabit,gigabits | B | 1000^3
Tb | Tb,terabit,terabits | B | 1000^4
Pb | Pb,petabit,petabits | B | 1000^5
Eb | Eb,exabit,exabits | B | 1000^6
Zb | Zb,zettabit,zettabits | B | 1000^7
Yb | Yb,yottabit,yottabits | B | 1000^8
Kib | Kib,kibibit,kibibits | B | 1024
Mib | Mib,mebibit,mebibits | B | 1024^2
Gib | Gib,gibibit,gibibits | B | 1024^3
Tib | Tib,tebibit,tebibits | B | 1024^4
Pib | Pib,pebibit,pebibits | B | 1024^5
Eib | Eib,exbibit,exbibits | B | 1024^6
Zib | Zib,zebibit,zebibits | B | 1024^7
Yib | Yib,yobibit,yobibits | B | 1024^8
# compound aliases
@ mph = mi/h
@ kph,kmh = km/h
@ kn,knot,knots = NM/h
@ rpm = rev/min
@ rps = rev/s
@ mpg,MPG = mi/gal
@ Bps = B/s
@ bps = bit/s
"#;

/// Words the phrase layer owns. A unit can show one as its symbol but is never found by it.
pub const RESERVED_WORDS: &[&str] = &[
    "in", "a", "as", "to", "is", "of", "on", "off", "what", "percent", "into",
];

const PREFIXES: &[(&str, &str, i32)] = &[
    ("y", "yocto", -24),
    ("z", "zepto", -21),
    ("a", "atto", -18),
    ("f", "femto", -15),
    ("p", "pico", -12),
    ("n", "nano", -9),
    ("µ", "micro", -6),
    ("m", "milli", -3),
    ("c", "centi", -2),
    ("d", "deci", -1),
    ("h", "hecto", 2),
    ("k", "kilo", 3),
    ("M", "mega", 6),
    ("G", "giga", 9),
    ("T", "tera", 12),
    ("P", "peta", 15),
    ("E", "exa", 18),
    ("Z", "zetta", 21),
    ("Y", "yotta", 24),
];

fn power_of_ten(exp: i32) -> String {
    if exp >= 0 {
        format!("1{}", "0".repeat(exp as usize))
    } else {
        format!("0.{}1", "0".repeat((-exp - 1) as usize))
    }
}

/// Evaluates `a*b/c`, `a^n` and one level of parentheses over decimal literals, exactly.
fn exact_factor(spec: &str) -> Number {
    let mut result = Number::from_i64(1);
    let mut dividing = false;
    let mut term = String::new();
    let flush = |term: &mut String, dividing: bool, result: &mut Number| {
        if term.is_empty() {
            return;
        }
        let t = term.trim_matches(|c| c == '(' || c == ')');
        let (base, exp) = t.split_once('^').map_or((t, 1), |(b, e)| {
            (b, e.trim_end_matches(')').parse().expect("factor exponent"))
        });
        let mut n =
            Number::from_decimal_str(base).unwrap_or_else(|e| panic!("bad factor {spec:?}: {e}"));
        let one = n.clone();
        for _ in 1..exp {
            n = n.mul(&one).expect("factor size");
        }
        *result = if dividing {
            result.div(&n)
        } else {
            result.mul(&n)
        }
        .expect("factor size");
        term.clear();
    };
    for c in spec.chars() {
        match c {
            '*' | '/' => {
                flush(&mut term, dividing, &mut result);
                dividing = c == '/';
            }
            c => term.push(c),
        }
    }
    flush(&mut term, dividing, &mut result);
    result
}

impl Registry {
    fn build() -> Registry {
        let mut reg = Registry {
            defs: Vec::new(),
            by_name: HashMap::new(),
        };
        let mut last: Option<(&str, Vec<&str>, Dim, Number)> = None;
        let mut aliases: Vec<(Vec<&str>, &str)> = Vec::new();
        for line in TABLE
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
        {
            if let Some(rest) = line.strip_prefix('@') {
                let (names, expr) = rest.split_once('=').expect("alias needs =");
                aliases.push((names.split(',').map(str::trim).collect(), expr.trim()));
            } else if let Some(prefixes) = line.strip_prefix('+') {
                let (symbol, names, dim, factor) =
                    last.clone().expect("prefix line needs a unit before it");
                for p in prefixes.split(',') {
                    let (psym, pname, exp) = *PREFIXES
                        .iter()
                        .find(|(s, ..)| *s == p)
                        .unwrap_or_else(|| panic!("prefix {p}"));
                    let f = factor
                        .mul(&exact_factor(&power_of_ten(exp)))
                        .expect("prefix factor");
                    let mut spellings: Vec<String> = vec![format!("{psym}{symbol}")];
                    if psym == "µ" {
                        spellings.push(format!("u{symbol}"));
                    }
                    for n in names
                        .iter()
                        .filter(|n| n.len() > 2 && n.chars().all(char::is_alphabetic))
                    {
                        spellings.push(format!("{pname}{n}"));
                    }
                    reg.add(
                        &format!("{psym}{symbol}"),
                        &spellings.iter().map(String::as_str).collect::<Vec<_>>(),
                        dim,
                        f,
                        true,
                    );
                }
            } else {
                let cols: Vec<&str> = line.split('|').map(str::trim).collect();
                assert!(cols.len() == 4 || cols.len() == 5, "bad unit line {line:?}");
                let names: Vec<&str> = cols[1].split(',').collect();
                let (dim, factor) = (Dim::parse(cols[2]), exact_factor(cols[3]));
                let offset = cols
                    .get(4)
                    .map_or_else(|| Number::from_i64(0), |o| exact_factor(o));
                reg.add(cols[0], &names, dim, factor.clone(), false);
                reg.defs.last_mut().expect("just added").offset = offset;
                last = Some((cols[0], names, dim, factor));
            }
        }
        for (names, expr) in aliases {
            let unit = reg.parse_symbols(expr);
            for n in names {
                reg.insert_name(n, unit.clone(), false);
            }
        }
        let squarable: Vec<(String, Vec<String>)> = reg
            .defs
            .iter()
            .enumerate()
            .filter(|(_, d)| d.dim == Dim::parse("L"))
            .map(|(i, d)| {
                let names: Vec<String> = reg
                    .by_name
                    .iter()
                    .filter(|(_, u)| u.0 == [(i as UnitId, 1)])
                    .map(|(n, _)| n.clone())
                    .collect();
                (d.symbol.to_string(), names)
            })
            .collect();
        for (symbol, names) in squarable {
            let id = reg
                .defs
                .iter()
                .position(|d| d.symbol == symbol)
                .expect("unit exists") as UnitId;
            for (prefix, word, exp) in [("sq", "square", 2), ("cu", "cubic", 3)] {
                let unit = UnitExpr(vec![(id, exp)]);
                if symbol.chars().all(|c| c.is_ascii_alphabetic()) {
                    reg.insert_name(&format!("{prefix}{symbol}"), unit.clone(), false);
                }
                for n in names
                    .iter()
                    .filter(|n| n.len() > 2 && n.chars().all(char::is_alphabetic))
                {
                    reg.insert_name(&format!("{word} {n}"), unit.clone(), false);
                }
                if symbol.chars().all(|c| c.is_ascii_alphabetic()) {
                    reg.insert_name(&format!("{word} {symbol}"), unit.clone(), false);
                }
            }
        }
        reg
    }

    fn add(&mut self, symbol: &str, names: &[&str], dim: Dim, factor: Number, generated: bool) {
        let id = self.defs.len() as UnitId;
        self.defs.push(UnitDef {
            symbol: Box::leak(symbol.to_string().into_boxed_str()),
            dim,
            factor,
            offset: Number::from_i64(0),
        });
        let unit = UnitExpr(vec![(id, 1)]);
        self.insert_name(symbol, unit.clone(), generated);
        for n in names {
            self.insert_name(n, unit.clone(), generated);
        }
    }

    fn insert_name(&mut self, name: &str, unit: UnitExpr, generated: bool) {
        if RESERVED_WORDS.contains(&name) {
            return;
        }
        match self.by_name.get(name) {
            None => {
                self.by_name.insert(name.to_string(), unit);
            }
            Some(existing) if *existing == unit => {}
            Some(_) if generated => {}
            Some(_) => panic!("unit name {name:?} is defined twice"),
        }
    }

    fn parse_symbols(&self, expr: &str) -> UnitExpr {
        let mut out = UnitExpr::default();
        let mut sign = 1i8;
        let mut word = String::new();
        let flush = |word: &mut String, sign: i8, out: &mut UnitExpr| {
            if word.is_empty() {
                return;
            }
            let u = self
                .by_name
                .get(word.as_str())
                .unwrap_or_else(|| panic!("alias uses unknown unit {word:?}"));
            *out = out.times(&u.pow(sign));
            word.clear();
        };
        for c in expr.chars() {
            match c {
                '*' => flush(&mut word, sign, &mut out),
                '/' => {
                    flush(&mut word, sign, &mut out);
                    sign = -1;
                }
                c => word.push(c),
            }
        }
        flush(&mut word, sign, &mut out);
        out
    }

    pub fn names(&self) -> impl Iterator<Item = (&String, &UnitExpr)> {
        self.by_name.iter()
    }
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

pub fn registry() -> &'static Registry {
    REGISTRY.get_or_init(Registry::build)
}

/// Finds a unit by its spelling. Case matters for symbols (`Mb` is not `MB`); long names also match any case.
pub fn lookup(name: &str) -> Option<UnitExpr> {
    let reg = registry();
    if let Some(u) = reg.by_name.get(name) {
        return Some(u.clone());
    }
    if name.len() >= 4 && name.chars().all(char::is_alphabetic) {
        return reg.by_name.get(&name.to_lowercase()).cloned();
    }
    None
}

impl UnitExpr {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn with_exponent(&self, exp: i8) -> UnitExpr {
        self.pow(exp)
    }

    pub fn unit_count(&self) -> usize {
        self.0.len()
    }

    fn pow(&self, n: i8) -> UnitExpr {
        UnitExpr(
            self.0
                .iter()
                .map(|(u, e)| (*u, e * n))
                .filter(|(_, e)| *e != 0)
                .collect(),
        )
    }

    pub fn times(&self, other: &UnitExpr) -> UnitExpr {
        let mut parts = self.0.clone();
        for (u, e) in &other.0 {
            match parts.iter_mut().find(|(pu, _)| pu == u) {
                Some((_, pe)) => *pe += e,
                None => parts.push((*u, *e)),
            }
        }
        parts.retain(|(_, e)| *e != 0);
        parts.sort_by_key(|(u, _)| *u);
        UnitExpr(parts)
    }

    /// True for a scale like °C whose zero is not absolute zero.
    pub fn has_offset(&self) -> bool {
        self.0
            .iter()
            .any(|(u, _)| !registry().defs[*u as usize].offset.is_zero())
    }

    /// True when this is exactly one unit to the first power, the only shape an offset scale can take.
    pub fn is_single(&self) -> bool {
        matches!(self.0.as_slice(), [(_, 1)])
    }

    /// Kelvin value of `n` of this offset unit.
    pub fn to_kelvin(&self, n: &Number) -> Result<Number, Error> {
        let (u, _) = self.0[0];
        let d = &registry().defs[u as usize];
        n.mul(&d.factor)?.add(&d.offset)
    }

    pub fn from_kelvin(&self, k: &Number) -> Result<Number, Error> {
        let (u, _) = self.0[0];
        let d = &registry().defs[u as usize];
        k.sub(&d.offset)?.div(&d.factor)
    }

    pub fn recip(&self) -> UnitExpr {
        self.pow(-1)
    }

    pub fn dim(&self) -> Dim {
        self.0.iter().fold(Dim::NONE, |d, (u, e)| {
            d.plus(registry().defs[*u as usize].dim.scaled(*e))
        })
    }

    /// Exact factor from this unit to SI base units.
    pub fn factor(&self) -> Result<Number, Error> {
        let mut f = Number::from_i64(1);
        for (u, e) in &self.0 {
            let base = &registry().defs[*u as usize].factor;
            let powered = base.pow(&Number::from_i64(i64::from(*e)))?;
            f = f.mul(&powered)?;
        }
        Ok(f)
    }

    /// Conventional text: `mm³`, `km/h`, `m/s²`.
    pub fn symbol(&self) -> String {
        let reg = registry();
        let part = |u: UnitId, e: i8| {
            let s = reg.defs[u as usize].symbol;
            if e == 1 {
                s.to_string()
            } else {
                format!("{s}{}", superscript(e))
            }
        };
        let up: Vec<String> = self
            .0
            .iter()
            .filter(|(_, e)| *e > 0)
            .map(|(u, e)| part(*u, *e))
            .collect();
        let down: Vec<String> = self
            .0
            .iter()
            .filter(|(_, e)| *e < 0)
            .map(|(u, e)| part(*u, -*e))
            .collect();
        match (up.is_empty(), down.len()) {
            (_, 0) => up.join("·"),
            (true, _) => self
                .0
                .iter()
                .map(|(u, e)| part_signed(reg.defs[*u as usize].symbol, *e))
                .collect::<Vec<_>>()
                .join("·"),
            (false, 1) => format!("{}/{}", up.join("·"), down[0]),
            (false, _) => format!("{}/({})", up.join("·"), down.join("·")),
        }
    }
}

fn part_signed(symbol: &str, e: i8) -> String {
    format!("{symbol}{}", superscript(e))
}

fn superscript(e: i8) -> String {
    const DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let mut out = String::new();
    if e < 0 {
        out.push('⁻');
    }
    for c in e.unsigned_abs().to_string().chars() {
        out.push(DIGITS[c.to_digit(10).unwrap_or(0) as usize]);
    }
    out
}

pub fn superscript_digit(c: char) -> Option<i8> {
    "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().position(|d| d == c).map(|p| p as i8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrase_words_are_never_units() {
        for w in RESERVED_WORDS {
            assert!(lookup(w).is_none(), "{w} must stay a phrase word");
        }
        assert_eq!(lookup("inch").unwrap().symbol(), "in");
        assert_eq!(lookup("sqin").unwrap().symbol(), "in²");
    }

    #[test]
    fn table_builds_without_duplicate_names() {
        assert!(registry().names().count() > 500);
    }

    #[test]
    fn exact_factor_handles_the_spec_grammar() {
        assert_eq!(
            exact_factor("0.0254/96"),
            Number::from_decimal_str("0.0254")
                .unwrap()
                .div(&Number::from_i64(96))
                .unwrap()
        );
        assert_eq!(exact_factor("8*1024^2"), Number::from_i64(8 * 1024 * 1024));
        assert_eq!(exact_factor("1000*2/(0.5^2)"), Number::from_i64(8000));
    }
}
