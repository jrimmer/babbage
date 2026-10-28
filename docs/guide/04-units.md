# Units and conversions

Write a number and a unit and Babbage knows what it is. Put `in`, `to` or `as` and another unit after it and you get a conversion. Units combine in sums and products the way you would expect, and speeds, prices per unit and fuel economy are units too.

![Conversions between lengths, weights, speeds, temperatures, times and data sizes](img/units.webp)

Every conversion factor is an exact fraction, so `1 mile in feet` is exactly `5,280 ft` and `3 kWh in MJ` is exactly `10.8 MJ`. Where the answer needs more digits than are shown, it carries the **≈** mark described in [Numbers and percentages](03-numbers-and-percentages.html#exact-or-approximate).

## Convert

Type the amount, a conversion word, and the unit you want.

| You type | Answer |
|---|---|
| `5 km in miles` | `3.1068559612 mi` |
| `5 km to miles` | `3.1068559612 mi` |
| `5 km as miles` | `3.1068559612 mi` |
| `6 feet in cm` | `182.88 cm` |
| `150 pounds in kg` | `68.0388555 kg` |
| `1 gallon in liters` | `3.785411784 L` |
| `3 hours to min` | `180 min` |

`in`, `to`, `as` and `into` all do the same thing. Names can be singular or plural, spelled out or short: `km`, `kilometer`, `kilometers` and `kilometre` are the same unit. The answer is written with the unit's short symbol, whatever you typed.

Because `in` is also the word for inches, spell the inch out when it is the target of a conversion: `72 inches in feet` is `6 ft`. Writing `12 in in cm` also works (`30.48 cm`), but `inch` and `inches` read better.

### Ask the other way round

Put the unit first and the quantity after to ask how many of a unit it holds:

| You type | Answer |
|---|---|
| `meters in 10 km` | `10,000 m` |
| `inches in 2 feet` | `24 in` |
| `seconds in a day` | `86,400 s` |
| `how many cm in 2 m` | `200 cm` |

### Several units at once

Join units with `and`, or put them one after another, to split an amount into parts:

| You type | Answer |
|---|---|
| `2.5 hours in hours and minutes` | `2 hours 30 min` |
| `10000 seconds in hours min s` | `2 hours 46 min 40 s` |
| `5.5 ft in feet and inches` | `5 ft 6 in` |
| `1.75 lb in lb oz` | `1 lb 12 oz` |
| `200 lb in st lb` | `14 st 4 lb` |

Going the other way, adjacent quantities are one amount: `5 feet 6 inches in cm` is `167.64 cm`, `5 lb 8 oz in kg` is `2.494758035 kg`, and `2 h 30 min in minutes` is `150 min`. Feet and inches typed together stay as words: `5 ft 6 in` shows `5 feet 6 inches`, and a part of a pound shows in ounces (`13.5 lb` is `13 lb 8 oz`).

### Pixels

Pixels, points, `em` and `rem` are lengths. A pixel is 1/96 inch, a point 1/72 inch, and an `em` or `rem` is 16 pixels, so `12 pt in px` is `16px` and `96 px in inches` is `1 in`. To count pixels at another density, add `at N ppi`: `1 in in px at 144 ppi` is `144px`.

## Units Babbage knows

| Family | Units, with examples |
|---|---|
| Length | `mm` `cm` `m` `km`, `inch` `ft` `yd` `mile`, `NM` (nautical mile), `furlong`, `fathom`, `light year`, `px` `pt` `em` `rem` |
| Area | `m²` (or `m2`, `square meters`), `sq ft`, `km²`, `acre`, `hectare` (`ha`), `are` |
| Volume | `L` `mL` `cL` `dL` `kL` (`ml`, `cl`, `dl` and `kl` work too), `m³`, `cup`, `tbsp`, `tsp`, `floz`, `pint`, `quart`, `gallon`, `bushel`, `acre foot`, imperial `imp gal` `imp pt` `imp fl oz` |
| Mass | `mg` `g` `kg`, `mcg`, `oz` `lb` `stone` `ton` (short), `tonne` (`t`), `troy ounce` (`oz t`), `carat` |
| Time | `ms` `µs` `ns`, `s` `min` `h` `day` `week` `month` `year`, `workday` |
| Temperature | `°C` (`C`, `celsius`), `°F` (`F`, `fahrenheit`), `K` (`kelvin`) |
| Speed | `km/h` (`kph`, `kmh`), `mph`, `m/s`, `knots`, any length per time |
| Fuel economy | `mpg`, `L/100km` |
| Angle | `deg` (`°`), `rad`, `rev`, `arcmin`, `arcsec` |
| Frequency | `Hz` `kHz` `MHz` `GHz`, `rpm`, `rps` |
| Energy | `J` `kJ` `MJ`, `Wh` `kWh` `MWh`, `cal`, `kcal` |
| Power | `mW` `W` `kW` `MW` `GW`, `hp` (mechanical), `metric hp` (`cv`), `ehp` |
| Force and pressure | `N` `kN`, `lbf`, `Pa` `kPa` `MPa`, `bar` `mbar`, `atm`, `psi` |
| Electrical | `V` `mV` `kV`, `A` `mA`, `Ω` `kΩ` `MΩ`, `Ah` `mAh`, `coulomb`, `farad` (`mF`, `uF` or `µF`), `henry` (`mH`, `uH` or `µH`), `siemens` (`mS`, `uS` or `µS`), `weber`, `tesla` (`mT`, `uT` or `µT`) |
| Chemistry | `mol` `mmol`, `molar` |
| Data | `bit`, `B` (byte), `kB` (or `KB`) `MB` `GB` `TB` `PB`, `KiB` `MiB` `GiB` `TiB`, `kb` `Mb` `Gb`, `Kib` `Mib` `Gib` |
| Data rates | `Mbps` `Gbps` `kbps` `bps`, `MBps` `GBps`, `MiBps`, or any data unit per time (`MB/s`) |
| Video | `frame`, `fps` |
| Money | one unit per currency; see [Money](05-money.html) |

Examples, each checked: `1 hp in W` is `745.6998715823 W`, `1 metric hp in W` is `735.49875 W`, `100 kPa in psi` is `14.503773773 psi`, `1 atm in bar` is `1.01325 bar`, `1 cal in J` is `4.184 J`, `90 deg in rad` is `1.5707963268 rad`, and `1 hectare in acres` is `2.4710538147 ac`.

Some details worth knowing:

- Prefixed liters can be written with a capital or a small `l`: `5 ml` and `5 mL` are both `5 mL`, and `cl`, `dl` and `kl` work the same way. `1.5 l` alone is fine, and `milliliters` and `millilitres` work in full.
- The electrical prefixes print as the SI name: `5 mF` is `5 mfarad`, `5 uF` and `5 µF` are `5 µfarad`, `5 mH` is `5 mhenry`, `5 mS` is `5 msiemens` and `5 mT` is `5 mtesla`.
- Gallons, quarts, pints, cups and fluid ounces are US measures. Put `imperial` or `uk` in front for the imperial ones: `1 imperial gallon in liters` is `4.54609 L`.
- `ton` is the short ton (2,000 lb) and `tonne` is 1,000 kg.
- `hp` is mechanical horsepower, exactly 550 ft·lbf per second. For the metric and electrical kinds write `metric hp` or `cv` (735.49875 W) and `ehp` (746 W).
- A `month` is 30.436875 days and a `year` is 365.2425 days, so `1 year in days` is `365.2425 days`. A `workday` is the work-hours setting, 8 hours unless you change it.
- Feet and inches can be written with marks: `5' 6"` is `5 feet 6 inches`, `6"` is `6 inches`, and `5' 6" + 2 inches` is `5 feet 8 inches`.
- `rpm` is revolutions per minute, so `60 rpm in rps` is `1 rev/s`.

## Sums and products

![Sums and products of units](img/unit-math.webp)

### A plain number takes the unit

A number beside a quantity in a sum becomes the same unit: `300 + 20 km` and `20 km + 300` are both `320 km`.

### Two units of one kind

In a sum, the result is in the larger unit: `1 km + 500 m` is `1.5 km`, `500 m + 1 km` is `1.5 km`, and `5 mi + 3 km` is `6.8641135767 mi`. A quotient of two quantities of one kind has no unit left: `10 km / 2 km` is `5`.

### Multiplying

A quantity times a plain number keeps its unit (`2 km * 3` is `6 km`). A length times a length is an area, in the left-hand unit squared: `10 m x 50 cm` is `5 m²`, `6 ft x 4 ft` is `24 ft²`, and `10 ft x 12 ft x 8 ft in m³` is `27.1841727283 m³`. Other products keep every unit as you typed it: `3 kg * 2 m` is `6 kg m`, `3 kg m/s` stays `3 kg m/s`, and `1 W * 3 h` is `3 W hour`. Convert it to what you want: `2 kW * 3 h in kWh` is `6 kWh`.

## Speeds and per-units

A speed is a length over a time, written with a slash, `per`, or the word `over`:

| You type | Answer |
|---|---|
| `100 m / 9.58 s` | `10.4384133612 m/s` |
| `100 km/h in mph` | `62.1371192237 mph` |
| `100 km/h in m/s` | `27.7777777778 m/s` |
| `60 miles per hour` | `60 mi/hour` |
| `10 km / 2 h` | `5 km/h` |
| `90 km over 3 days` | `30 km/day` |
| `50 knots in km/h` | `92.6 km/h` |

The unit comes back the way you wrote the hour. `h` stays `km/h`, while `hours` and `hr` after a slash read `km/hour`. Use a rate in a product and the time cancels. A bare unit after a slash belongs to the factor just before it, so `40 hours * $25 / hour` is `$1,000.00`, the same as `$25/hour * 40 hours`:

| You type | Answer |
|---|---|
| `12 km/h * 90 min` | `18 km` |
| `5 km/h * 2 h` | `10 km` |
| `50 mph * 3 hours` | `150 mi` |
| `300 miles / 60 mph` | `5 hours` |
| `300 miles at 60 mph` | `5 hours` |
| `5 km / 20 min in km/h` | `15 km/h` |

`at` after a distance gives the time it takes. The same pattern prices things per unit; see [Money](05-money.html#price-per-unit).

### Fuel economy

Fuel economy is distance per volume (`mpg`, US gallons) or volume per distance (`L/100km`). They convert into each other:

| You type | Answer |
|---|---|
| `30 mpg in L/100km` | `7.8404861111 L/100km` |
| `8 L/100km in mpg` | `29.4018229167 MPG` |

Multiply or divide fuel economy by what it measures and the units cancel:

| You type | Answer |
|---|---|
| `10 mpg * 3 gallons` | `30 mi` |
| `300 mi / 30 mpg` | `10 gal` |
| `8 L/100km * 250 km` | `20 L` |

## Temperatures

Convert with `C`, `F` and `K` (or `°C`, `°F`, `celsius`, `fahrenheit`, `kelvin`):

| You type | Answer |
|---|---|
| `20 C in F` | `68 °F` |
| `98.6 F in C` | `37 °C` |
| `-40 F in C` | `-40 °C` |
| `100 C in fahrenheit` | `212 °F` |
| `0 C in K` | `273.15 K` |
| `300 K in C` | `26.85 °C` |
| `70 F to celsius` | `21.1111111111 °C` |

A temperature is a point on a scale, so there are only a few things you can do with one besides converting it. You can add or subtract a plain number, a percentage or a temperature of the same scale, and multiply or divide by a plain number:

| You type | Answer |
|---|---|
| `20 C + 5` | `25 °C` |
| `25 C - 5 C` | `20 °C` |
| `20 C * 2` | `40 °C` |
| `20 C + 10%` | `22 °C` |

Mixing two scales (`20 C + 5 F`) is refused. Convert one first.

## Data sizes

Babbage keeps the decimal and binary families apart, as the names do.

| Family | Units | Each step |
|---|---|---|
| Decimal bytes | `kB` `MB` `GB` `TB` `PB` | 1,000 |
| Binary bytes | `KiB` `MiB` `GiB` `TiB` `PiB` | 1,024 |
| Decimal bits | `kb` `Mb` `Gb` `Tb` | 1,000 |
| Binary bits | `Kib` `Mib` `Gib` `Tib` | 1,024 |

A byte is 8 bits. Case matters: `MB` is megabytes and `Mb` is megabits. The kilobyte is written `kB`, and `KB` is read as the same unit: `5 KB` is `5 kB`.

| You type | Answer |
|---|---|
| `1 GB in MB` | `1,000 MB` |
| `1 GiB in MiB` | `1,024 MiB` |
| `1 GB in GiB` | `0.9313225746 GiB` |
| `1.5 TB in GiB` | `1,396.9838619232 GiB` |
| `1 byte in bits` | `8 bit` |
| `5 Mb in kB` | `625 kB` |
| `1 TB in GB` | `1,000 GB` |

### Transfer speeds and times

`Mbps` is megabits per second and `MBps` (or `MB/s`) is megabytes per second. Divide a size by a rate to get a time:

| You type | Answer |
|---|---|
| `100 Mbps in MB/s` | `12.5 MB/s` |
| `1 GB / 100 Mbps` | `80 s` |
| `1 GB at 100 Mbps` | `80 s` |
| `20 GB at 50 MBps` | `400 s` |

## Mixing units and words

A word that is not a unit is skipped, so `10 km walked` is `10 km`. A word right after a plain number that is one letter away from a real unit gets a wavy underline, and the pointer over it shows "Did you mean km?" (`5 kmm`). If two units measure different things, the line says so: `1 kg in km` is an error ("these units measure different things"). A unit Babbage does not know is skipped like any other word, and then the conversion is not done, so check that the answer ends in the unit you asked for.

Units also work inside [variables and totals](06-variables-and-totals.html) and with [dates and times](07-dates-and-times.html).
