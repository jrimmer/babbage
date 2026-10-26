use std::collections::HashMap;
use std::sync::OnceLock;

use crate::error::Error;
use crate::number::Number;

const BASES: usize = 9;

/// Exponents of the base dimensions: length, mass, time, temperature, current, amount, data (bit), angle.
/// Money is separate: each currency family (USD, EUR, BTC...) is its own dimension, so two currencies
/// never convert into each other without an exchange rate.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Dim {
    exps: [i8; BASES],
    money: Vec<(u16, i8)>,
}

impl Dim {
    pub fn none() -> Dim {
        Dim {
            exps: [0; BASES],
            money: Vec::new(),
        }
    }

    fn parse(spec: &str) -> Dim {
        let mut d = Dim::none();
        for part in spec.split_whitespace() {
            let (letter, exp) = part.split_at(1);
            let slot = "LMTKINBAF"
                .find(letter)
                .unwrap_or_else(|| panic!("bad dimension letter in {spec:?}"));
            d.exps[slot] = if exp.is_empty() {
                1
            } else {
                exp.parse().expect("dimension exponent")
            };
        }
        d
    }

    pub fn is_none(&self) -> bool {
        self.exps == [0; BASES] && self.money.is_empty()
    }

    pub fn has_money(&self) -> bool {
        !self.money.is_empty()
    }

    fn scaled(&self, by: i8) -> Dim {
        let mut d = self.clone();
        for e in &mut d.exps {
            *e = e.saturating_mul(by);
        }
        for (_, e) in &mut d.money {
            *e = e.saturating_mul(by);
        }
        d.money.retain(|(_, e)| *e != 0);
        d
    }

    fn plus(&self, other: &Dim) -> Dim {
        let mut d = self.clone();
        for (e, o) in d.exps.iter_mut().zip(other.exps) {
            *e = e.saturating_add(o);
        }
        for (family, exp) in &other.money {
            match d.money.iter_mut().find(|(f, _)| f == family) {
                Some((_, e)) => *e = e.saturating_add(*exp),
                None => d.money.push((*family, *exp)),
            }
        }
        d.money.retain(|(_, e)| *e != 0);
        d.money.sort_unstable();
        d
    }
}

pub type UnitId = u16;

struct UnitDef {
    symbol: &'static str,
    dim: Dim,
    factor: Number,
    /// Kelvin at zero of this unit, for scales that do not start at absolute zero.
    offset: Number,
    currency: Option<Currency>,
    /// The factor is a close rational stand-in for an irrational number (radians).
    approximate: bool,
}

/// How a currency is written and shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Currency {
    pub code: &'static str,
    /// True when the symbol goes before the number (`$5.00`); otherwise the code follows it (`5.00 CHF`).
    pub prefix_symbol: bool,
    /// Digits after the point; None for crypto and commodities, which show every digit.
    pub minor_digits: Option<u32>,
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
workday | workday,workdays,work day,work days | T | 8*3600
mo | mo,month,months | T | 30.436875*86400
yr | yr,yrs,year,years | T | 365.2425*86400
# video
frame | frame,frames | F | 1
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
hp | hp,horsepower | M L2 T-3 | 550*0.3048*0.45359237*9.80665
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
@ fps = frame/s
@ rpm = rev/min
@ rps = rev/s
@ mpg,MPG = mi/gal
@ Bps = B/s
@ bps = bit/s
"#;

/// Words the phrase layer owns. A unit can show one as its symbol but is never found by it.
pub const RESERVED_WORDS: &[&str] = &[
    "in", "a", "as", "to", "is", "of", "on", "off", "what", "percent", "into", "cast", "at",
];

/// ISO 4217 codes from the reference, with the digits after the point where they are not 2.
const FIAT: &str = "AED AFN ALL AMD ANG AOA ARS AUD AWG AZN BAM BBD BDT BGN BHD:3 BIF:0 BMD BND BOB BRL BSD BTN BWP BYN BZD \
CAD CDF CHF CLF:4 CLP:0 CNY COP CRC CUC CVE CZK DJF:0 DKK DOP DZD EGP ERN ETB EUR FJD FKP GBP GEL GGP GHS GIP GMD GNF:0 \
GTQ GYD HKD HNL HRK HTG HUF IDR ILS IMP INR IQD:3 IRR ISK:0 JEP JMD JOD:3 JPY:0 KES KGS KHR KMF:0 KPW KRW:0 KWD:3 KYD KZT \
LAK LBP LKR LRD LSL LTL LVL LYD:3 MAD MDL MGA MKD MMK MNT MOP MRU MUR MVR MWK MXN MYR MZN NAD NGN NIO NOK NPR NZD OMR:3 \
PAB PEN PGK PHP PKR PLN PYG:0 QAR RON RSD RUB RWF:0 SAR SBD SCR SDG SEK SGD SHP SLL SOS SRD STD SVC SYP SZL THB TJS TMT \
TND:3 TOP TRY TTD TWD TZS UAH UGX:0 USD UYU UZS VEF VND:0 VUV:0 WST XAF:0 XCD XDR XOF:0 XPF:0 YER ZAR ZMW ZWL";

/// Crypto-currencies and commodities show every digit instead of a fixed number of places.
const FREE: &str = "BTC ETH ADA AVAX BCH BNB BSV DASH DOGE DOT EOS ETC LTC LUNA POL MATIC NEO SHIB SOL TRX USDT XLM XMR XRP XAG XAU";

/// (code, symbol written before the amount).
const CURRENCY_SYMBOLS: &[(&str, &str)] = &[
    ("USD", "$"),
    ("EUR", "€"),
    ("GBP", "£"),
    ("JPY", "¥"),
    ("INR", "₹"),
    ("KRW", "₩"),
    ("RUB", "₽"),
    ("TRY", "₺"),
    ("ILS", "₪"),
    ("VND", "₫"),
    ("THB", "฿"),
    ("PHP", "₱"),
    ("UAH", "₴"),
    ("NGN", "₦"),
    ("AUD", "A$"),
    ("CAD", "C$"),
    ("NZD", "NZ$"),
    ("SGD", "S$"),
    ("HKD", "HK$"),
    ("TWD", "NT$"),
    ("BRL", "R$"),
];

/// Ways to write a currency symbol in front of a number, longest first.
const SYMBOL_INPUTS: &[(&str, &str)] = &[
    ("US$", "USD"),
    ("NZ$", "NZD"),
    ("CA$", "CAD"),
    ("AU$", "AUD"),
    ("HK$", "HKD"),
    ("NT$", "TWD"),
    ("C$", "CAD"),
    ("A$", "AUD"),
    ("S$", "SGD"),
    ("R$", "BRL"),
    ("$", "USD"),
    ("€", "EUR"),
    ("£", "GBP"),
    ("¥", "JPY"),
    ("₹", "INR"),
    ("₩", "KRW"),
    ("₽", "RUB"),
    ("₺", "TRY"),
    ("₪", "ILS"),
    ("₫", "VND"),
    ("฿", "THB"),
    ("₱", "PHP"),
    ("₴", "UAH"),
    ("₦", "NGN"),
];

const PREFIXES: &[(&str, &str, i32)] = &[
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
                        dim.clone(),
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
                reg.add(cols[0], &names, dim.clone(), factor.clone(), false);
                reg.defs.last_mut().expect("just added").offset = offset;
                last = Some((cols[0], names, dim, factor));
            }
        }
        reg.add(
            "rad",
            &["rad", "radian", "radians"],
            Dim::parse("A"),
            crate::approx::degrees_per_radian(),
            false,
        );
        reg.defs.last_mut().expect("just added").approximate = true;
        reg.add_currencies();
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

    fn add_currencies(&mut self) {
        let mut family_of: HashMap<&str, u16> = HashMap::new();
        let mut next_family = 0u16;
        let define = |reg: &mut Registry,
                      code: &'static str,
                      minor: Option<u32>,
                      family: u16,
                      factor: &str,
                      names: &[&str]| {
            let symbol = CURRENCY_SYMBOLS
                .iter()
                .find(|(c, _)| *c == code)
                .map(|(_, s)| *s);
            let dim = Dim {
                exps: [0; BASES],
                money: vec![(family, 1)],
            };
            reg.add(
                symbol.unwrap_or(code),
                &[],
                dim,
                exact_factor(factor),
                false,
            );
            let id = (reg.defs.len() - 1) as UnitId;
            reg.defs[id as usize].currency = Some(Currency {
                code,
                prefix_symbol: symbol.is_some(),
                minor_digits: minor,
            });
            let unit = UnitExpr(vec![(id, 1)]);
            reg.insert_name(code, unit.clone(), false);
            for n in names {
                reg.insert_name(n, unit.clone(), false);
            }
        };
        for entry in FIAT.split_whitespace() {
            let (code, minor) = match entry.split_once(':') {
                Some((c, d)) => (c, Some(d.parse().expect("minor digits"))),
                None => (entry, Some(2)),
            };
            let family = next_family;
            next_family += 1;
            family_of.insert(code, family);
            let names: &[&str] = match code {
                "USD" => &["dollar", "dollars"],
                "EUR" => &["euro", "euros"],
                "JPY" => &["yen"],
                _ => &[],
            };
            define(self, code, minor, family, "1", names);
        }
        for code in FREE.split_whitespace() {
            let family = next_family;
            next_family += 1;
            family_of.insert(code, family);
            let names: &[&str] = match code {
                "BTC" => &["bitcoin", "bitcoins"],
                "ETH" => &["ether"],
                _ => &[],
            };
            define(self, code, None, family, "1", names);
        }
        define(
            self,
            "SAT",
            None,
            family_of["BTC"],
            "0.00000001",
            &["Satoshi", "satoshi", "satoshis"],
        );
        define(
            self,
            "WEI",
            None,
            family_of["ETH"],
            "0.000000000000000001",
            &["Wei", "wei"],
        );
        define(
            self,
            "GWEI",
            None,
            family_of["ETH"],
            "0.000000001",
            &["Gwei", "gwei"],
        );
    }

    fn add(&mut self, symbol: &str, names: &[&str], dim: Dim, factor: Number, generated: bool) {
        let id = self.defs.len() as UnitId;
        self.defs.push(UnitDef {
            symbol: Box::leak(symbol.to_string().into_boxed_str()),
            dim,
            factor,
            offset: Number::from_i64(0),
            currency: None,
            approximate: false,
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

/// A currency symbol at the start of `text` (`$`, `US$`, `€`...), with its byte length.
pub fn currency_prefix(text: &str) -> Option<(usize, UnitExpr)> {
    SYMBOL_INPUTS
        .iter()
        .find(|(symbol, _)| text.starts_with(symbol))
        .and_then(|(symbol, code)| {
            let code = if *symbol == "$" {
                crate::settings::dollar_code()
            } else {
                code
            };
            lookup(code).map(|u| (symbol.len(), u))
        })
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

/// Calendar units that count in working time when a workday is involved: a week is five workdays, so a day is
/// five sevenths of one.
fn is_calendar_unit(symbol: &str) -> bool {
    matches!(symbol, "day" | "wk" | "mo" | "yr")
}

/// The factor of a unit to SI when `workday_context` says a workday is on the other side or in the same
/// expression: calendar units are scaled to working time so that a week is exactly five workdays.
fn factor_in_context(def: &UnitDef, workday_context: bool) -> Number {
    let base = factor_now(def);
    if workday_context && is_calendar_unit(def.symbol) {
        let hours = i64::from(crate::settings::work_hours_per_day());
        let scale = Number::from_i64(5 * hours)
            .div(&Number::from_i64(7 * 24))
            .expect("seven days of 24 hours is not zero");
        base.mul(&scale).unwrap_or(base)
    } else {
        base
    }
}

/// A unit's factor to SI under the current settings: a workday is the work-hours setting, and the larger
/// US volume units become the imperial ones when the volume system says so.
fn factor_now(def: &UnitDef) -> Number {
    use crate::settings::VolumeSystem;
    match def.symbol {
        "workday" => Number::from_i64(i64::from(crate::settings::work_hours_per_day()) * 3600),
        "gal" | "qt" | "pint" | "floz" | "tbsp" | "tsp"
            if crate::settings::volume_system() == VolumeSystem::Imperial =>
        {
            // The imperial fluid ounce is a hundred and sixtieth of the gallon; the tablespoon is five
            // eighths of it and the teaspoon a third of the tablespoon. The cup stays US customary.
            let (numerator, divisor) = match def.symbol {
                "gal" => (1, 1),
                "qt" => (1, 4),
                "pint" => (1, 8),
                "floz" => (1, 160),
                "tbsp" => (5, 160 * 8),
                _ => (5, 160 * 8 * 3),
            };
            Number::from_decimal_str("0.00454609")
                .and_then(|gallon| gallon.mul(&Number::from_i64(numerator)))
                .and_then(|n| n.div(&Number::from_i64(divisor)))
                .expect("the imperial gallon is a plain decimal")
        }
        _ => def.factor.clone(),
    }
}

impl UnitExpr {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn with_exponent(&self, exp: i8) -> UnitExpr {
        self.pow(exp)
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

    /// The currency when this is exactly one currency unit, as in `$` or `CHF`.
    pub fn single_currency(&self) -> Option<Currency> {
        match self.0.as_slice() {
            [(u, 1)] => registry().defs[*u as usize].currency,
            _ => None,
        }
    }

    /// For a rate such as `$/day`: the currency and the remaining units, when the money has exponent 1.
    pub fn currency_split(&self) -> Option<(Currency, UnitExpr)> {
        let (money, rest): (Vec<_>, Vec<_>) = self
            .0
            .iter()
            .copied()
            .partition(|(u, _)| registry().defs[*u as usize].currency.is_some());
        match money.as_slice() {
            [(u, 1)] => Some((registry().defs[*u as usize].currency?, UnitExpr(rest))),
            _ => None,
        }
    }

    /// The symbol of the single currency in this expression, as written before an amount.
    pub fn currency_symbol(&self) -> Option<&'static str> {
        self.0.iter().find_map(|(u, _)| {
            let d = &registry().defs[*u as usize];
            d.currency.filter(|c| c.prefix_symbol).map(|c| {
                let region_dollar = crate::settings::dollar_code();
                if c.code == region_dollar && crate::settings::REGION_DOLLARS.contains(&c.code) {
                    "$"
                } else if c.code == "USD" {
                    "US$"
                } else {
                    d.symbol
                }
            })
        })
    }

    /// Cancels pairs such as `week/day` or `km/m` by converting the number. Returns the scale to apply to
    /// the number and the smaller expression.
    pub fn reduce(&self) -> Result<(Number, UnitExpr), Error> {
        let mut parts = self.0.clone();
        let mut scale = Number::from_i64(1);
        let workday_context = self.contains_workday();
        'again: loop {
            for i in 0..parts.len() {
                for j in 0..parts.len() {
                    let (u, e) = parts[i];
                    let (v, f) = parts[j];
                    let (du, dv) = (&registry().defs[u as usize], &registry().defs[v as usize]);
                    if e > 0
                        && f < 0
                        && du.dim == dv.dim
                        && du.offset.is_zero()
                        && dv.offset.is_zero()
                    {
                        let k = e.min(-f);
                        let ratio = factor_in_context(du, workday_context)
                            .div(&factor_in_context(dv, workday_context))?
                            .pow(&Number::from_i64(i64::from(k)))?;
                        scale = scale.mul(&ratio)?;
                        parts[i].1 -= k;
                        parts[j].1 += k;
                        parts.retain(|(_, x)| *x != 0);
                        continue 'again;
                    }
                }
            }
            break;
        }
        parts.sort_by_key(|(u, _)| *u);
        Ok((scale, UnitExpr(parts)))
    }

    /// True when a conversion through this expression uses an approximate factor.
    pub fn is_approximate(&self) -> bool {
        self.0
            .iter()
            .any(|(u, _)| registry().defs[*u as usize].approximate)
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

    /// The units with positive exponents, as in `$` of `$/hour`.
    pub fn numerator(&self) -> UnitExpr {
        UnitExpr(
            self.0
                .iter()
                .copied()
                .filter(|(_, e)| e.is_positive())
                .collect(),
        )
    }

    /// The units with negative exponents, turned positive, as in `hour` of `$/hour`.
    pub fn denominator(&self) -> UnitExpr {
        UnitExpr(
            self.0
                .iter()
                .copied()
                .filter(|(_, e)| e.is_negative())
                .map(|(u, e)| (u, -e))
                .collect(),
        )
    }

    pub fn recip(&self) -> UnitExpr {
        self.pow(-1)
    }

    pub fn dim(&self) -> Dim {
        self.0.iter().fold(Dim::none(), |d, (u, e)| {
            d.plus(&registry().defs[*u as usize].dim.scaled(*e))
        })
    }

    /// This rate with its period (the one time unit in the denominator) replaced by `period`.
    pub fn with_period(&self, period: &UnitExpr) -> Option<UnitExpr> {
        let time = lookup("s")?.dim();
        let reg = registry();
        let [(new, 1)] = period.0.as_slice() else {
            return None;
        };
        if reg.defs[*new as usize].dim != time {
            return None;
        }
        let mut changed = false;
        let mut parts: Vec<(UnitId, i8)> = Vec::new();
        for (u, e) in &self.0 {
            if *e == -1 && reg.defs[*u as usize].dim == time && !changed {
                parts.push((*new, -1));
                changed = true;
            } else {
                parts.push((*u, *e));
            }
        }
        changed.then(|| {
            parts.sort_by_key(|(u, _)| *u);
            UnitExpr(parts)
        })
    }

    /// True when some unit is divided by another, as in `$/day` or `km/h`.
    pub fn has_denominator(&self) -> bool {
        self.0.iter().any(|(_, e)| e.is_negative())
    }

    /// True for one unit of time over another, such as `hours/day`.
    pub fn is_time_over_time(&self) -> bool {
        let Some(seconds) = lookup("s") else {
            return false;
        };
        let (time, reg) = (seconds.dim(), registry());
        match self.0.as_slice() {
            [(a, ea), (b, eb)] => {
                reg.defs[*a as usize].dim == time
                    && reg.defs[*b as usize].dim == time
                    && ea + eb == 0
            }
            _ => false,
        }
    }

    /// True when a workday is part of this expression.
    pub fn contains_workday(&self) -> bool {
        self.0
            .iter()
            .any(|(u, _)| registry().defs[*u as usize].symbol == "workday")
    }

    /// Exact factor from this unit to SI base units.
    pub fn factor(&self) -> Result<Number, Error> {
        self.factor_in(false)
    }

    /// The factor to use when this unit is set against `other`: where either holds a workday, calendar
    /// units are counted in working time (a week is five workdays).
    pub fn factor_against(&self, other: &UnitExpr) -> Result<Number, Error> {
        self.factor_in(self.contains_workday() || other.contains_workday())
    }

    fn factor_in(&self, workday_context: bool) -> Result<Number, Error> {
        let mut f = Number::from_i64(1);
        for (u, e) in &self.0 {
            let def = &registry().defs[*u as usize];
            let base = factor_in_context(def, workday_context);
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
            .filter(|(_, e)| e.is_positive())
            .map(|(u, e)| part(*u, *e))
            .collect();
        let down: Vec<String> = self
            .0
            .iter()
            .filter(|(_, e)| e.is_negative())
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

/// Units an answer writes as words, as the reference app does: `3 hours/day`, `$99.00/week`, `5,400 frames`.
/// A word is plural when the amount is not one and singular after a slash. Minutes and seconds stay `min` and `s`.
const WORD_UNITS: &[(&str, &str)] = &[
    ("h", "hour"),
    ("day", "day"),
    ("wk", "week"),
    ("mo", "month"),
    ("yr", "year"),
    ("workday", "workday"),
    ("frame", "frame"),
];

impl UnitExpr {
    /// How this unit reads in an answer. `plural` is whether the amount in front of it is not one.
    pub fn display(&self, plural: bool) -> String {
        let reg = registry();
        let name = |u: UnitId| -> (String, bool) {
            let symbol = reg.defs[u as usize].symbol;
            match WORD_UNITS.iter().find(|(s, _)| *s == symbol) {
                Some((_, word)) => ((*word).to_string(), true),
                None => (symbol.to_string(), false),
            }
        };
        let part = |u: UnitId, e: i8, may_pluralise: bool| {
            let (text, is_word) = name(u);
            if e == 1 {
                if is_word && may_pluralise && plural {
                    format!("{text}s")
                } else {
                    text
                }
            } else {
                format!("{text}{}", superscript(e))
            }
        };
        let up: Vec<String> = self
            .0
            .iter()
            .filter(|(_, e)| e.is_positive())
            .enumerate()
            .map(|(i, (u, e))| part(*u, *e, i == 0))
            .collect();
        let down: Vec<(UnitId, i8)> = self
            .0
            .iter()
            .filter(|(_, e)| e.is_negative())
            .map(|(u, e)| (*u, -*e))
            .collect();
        // A speed such as `km/h` keeps its symbols: words are for rates of money, counts and times.
        let speed_like = !self.0.iter().any(|(u, e)| {
            e.is_positive() && {
                let def = &reg.defs[*u as usize];
                def.currency.is_some() || WORD_UNITS.iter().any(|(s, _)| *s == def.symbol)
            }
        }) && !up.is_empty();
        let down_text: Vec<String> = down
            .iter()
            .map(|(u, e)| {
                if speed_like {
                    let s = reg.defs[*u as usize].symbol;
                    if *e == 1 {
                        s.to_string()
                    } else {
                        format!("{s}{}", superscript(*e))
                    }
                } else {
                    part(*u, *e, false)
                }
            })
            .collect();
        match (up.is_empty(), down.len()) {
            (_, 0) => up.join("·"),
            // Only a denominator (`30/week`): written after a slash when each is to the first power.
            (true, 1) if down[0].1 == 1 => format!("/{}", down_text[0]),
            (true, _) => self.symbol(),
            (false, 1) => format!("{}/{}", up.join("·"), down_text[0]),
            (false, _) => format!("{}/({})", up.join("·"), down_text.join("·")),
        }
    }
}

fn part_signed(symbol: &str, e: i8) -> String {
    format!("{symbol}{}", superscript(e))
}

fn superscript(e: i8) -> String {
    const DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let mut out = String::new();
    if e.is_negative() {
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

    fn empty_registry() -> Registry {
        Registry {
            defs: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    fn one(id: UnitId) -> UnitExpr {
        UnitExpr(vec![(id, 1)])
    }

    #[test]
    fn registering_the_same_name_for_the_same_unit_is_a_no_op() {
        let mut reg = empty_registry();
        reg.insert_name("m", one(1), false);
        reg.insert_name("m", one(1), false);
        reg.insert_name("m", one(1), true);
        assert_eq!(reg.by_name["m"], one(1));
    }

    #[test]
    fn a_generated_name_never_replaces_an_explicit_one() {
        let mut reg = empty_registry();
        reg.insert_name("min", one(1), false);
        reg.insert_name("min", one(2), true);
        assert_eq!(reg.by_name["min"], one(1));
        reg.insert_name("fresh", one(3), true);
        assert_eq!(reg.by_name["fresh"], one(3));
    }

    #[test]
    #[should_panic(expected = "defined twice")]
    fn two_explicit_units_cannot_share_a_name() {
        let mut reg = empty_registry();
        reg.insert_name("min", one(1), false);
        reg.insert_name("min", one(2), false);
    }

    #[test]
    fn reserved_words_are_skipped_silently() {
        let mut reg = empty_registry();
        reg.insert_name("in", one(1), false);
        assert!(reg.by_name.is_empty());
    }

    #[test]
    fn single_units_are_told_apart_from_compound_ones() {
        assert!(lookup("km").unwrap().is_single());
        assert!(!lookup("mph").unwrap().is_single());
        assert!(!lookup("sqft").unwrap().is_single());
        assert!(
            !lookup("m")
                .unwrap()
                .times(&lookup("s").unwrap())
                .is_single()
        );
        assert!(!UnitExpr::default().is_single());
    }

    #[test]
    fn long_names_match_in_any_case_but_symbols_do_not() {
        assert_eq!(lookup("Meters"), lookup("meters"));
        assert_eq!(lookup("KILOMETERS"), lookup("km"));
        assert!(lookup("KM").is_none());
        assert!(lookup("Km").is_none());
        assert!(lookup("MB").is_some() && lookup("MB") != lookup("Mb"));
        assert!(lookup("mB").is_none());
        assert!(lookup("Day1").is_none());
        assert!(lookup("abc").is_none());
        assert!(lookup("DAY").is_none());
        assert!(lookup("DAYS").is_some());
        assert!(lookup("Hrs").is_none());
    }

    #[test]
    fn symbols_read_like_the_reference() {
        let sym = |a: &str| lookup(a).unwrap().symbol();
        assert_eq!(sym("km"), "km");
        assert_eq!(sym("sqft"), "ft²");
        assert_eq!(sym("cuft"), "ft³");
        assert_eq!(sym("mph"), "mi/h");
        assert_eq!(sym("rpm"), "rev/min");
        let m = lookup("m").unwrap();
        let s = lookup("s").unwrap();
        let kg = lookup("kg").unwrap();
        assert_eq!(m.times(&s.recip()).symbol(), "m/s");
        assert_eq!(m.times(&s.with_exponent(-2)).symbol(), "m/s²");
        assert_eq!(kg.times(&m).times(&s.with_exponent(-2)).symbol(), "m·kg/s²");
        assert_eq!(s.recip().symbol(), "s⁻¹");
        assert_eq!(m.with_exponent(-1).times(&s.recip()).symbol(), "s⁻¹·m⁻¹");
        assert_eq!(m.times(&s.recip()).times(&kg.recip()).symbol(), "m/(s·kg)");
        assert_eq!(m.with_exponent(10).symbol(), "m¹⁰");
        assert_eq!(UnitExpr::default().symbol(), "");
    }

    #[test]
    fn superscript_digits_cover_zero_to_nine() {
        for (i, c) in "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().enumerate() {
            assert_eq!(superscript_digit(c), Some(i as i8));
        }
        assert_eq!(superscript_digit('9'), None);
        assert_eq!(superscript_digit('a'), None);
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
