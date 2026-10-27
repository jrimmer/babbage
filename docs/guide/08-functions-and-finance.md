# Functions and finance

Babbage has the usual math functions, written either as `name(value)` or in words (`square root of 16`), plus loan, interest and growth calculations that read like sentences. Answers are exact where they can be: `sqrt(16)` is `4` and `1/3` stays a true third until it has to be shown as a decimal.

![Roots, trigonometry, factorial, a loan payment and compound growth](img/functions-finance.webp)

Function names are not case sensitive. A function takes its values in brackets separated by commas (`gcd(20, 30)`), and many also accept a list in words (`gcd of 20 and 30`). Where two values are needed, the order is the one in the table.

## Powers and roots

| You type | Answer |
|---|---|
| `2 ^ 10` | `1,024` |
| `2 ** 10` | `1,024` |
| `10 ^ -2` | `0.01` |
| `2 ^ 3 ^ 2` | `512` (powers group from the right) |
| `5 squared` | `25` |
| `2 cubed` | `8` |
| `2 to the power of 10` | `1,024` |
| `sqrt(16)`, `sqrt 16`, `square root of 16` | `4` |
| `sqrt(2)` | `1.4142135624` |
| `cbrt(27)`, `cube root of 27` | `3` |
| `5 root of 32`, `root 5 of 32` | `2` |
| `root 5 of 100` | `2.5118864315` |

A fraction as the exponent is a root: `4 ^ 0.5` is `2`, `8 ^ (1/3)` is `2`, `100 ^ 1.5` is `1,000` and `2 ^ 0.5` is `1.4142135624`. `sqrt` for a half, `cbrt` for a third and `root N of` for any other root say the same thing in words. A negative base with a fractional exponent, `sqrt(-1)` and other out-of-range values answer with an error rather than a guess.

## Logarithms and exponentials

| You type | Answer |
|---|---|
| `ln(10)` | `2.302585093` |
| `log(1000)`, `log10(1000)` | `3` (`log` is base 10) |
| `log2(8)` | `3` |
| `log 20 base 4` | `2.1609640474` |
| `81 is 9 to what power` | `2` |
| `exp(1)` | `2.7182818285` |

## Constants

| You type | Answer |
|---|---|
| `pi` or `π` | `3.1415926536` |
| `tau` or `τ` | `6.2831853072` |
| `phi` or `φ` | `1.6180339887` |

A number straight before a constant multiplies it, so `2pi` and `2 pi` are both `6.2831853072`. For Euler's number use `exp(1)`: a bare `e` is not a constant.

## Trigonometry

Plain `sin`, `cos` and `tan` read their value in radians. Put a degree sign or the word `degrees` on the value, or use the `d` forms, to work in degrees.

| You type | Answer |
|---|---|
| `sin(30)` | `-0.9880316241` (30 radians) |
| `sin(30°)`, `sin(30 degrees)`, `sind(30)` | `0.5` |
| `cosd(60)` | `0.5` |
| `tand(45)` | `1` |
| `sin(pi/2)` | `1` |
| `cos(pi)` | `-1` |
| `sin(0.5 rad)` | `0.4794255386` |
| `asin(1)` | `1.5707963268` (radians) |
| `asind(0.5)` | `30°` |
| `acosd(0.5)` | `60°` |
| `atand(1)` | `45°` |
| `1 rad in degrees` | `57.2957795131°` |
| `90° in radians` | `1.5707963268 rad` |

Every family comes in the three spellings above.

| Radians | Degrees | Inverse (radians) | Inverse (degrees) |
|---|---|---|---|
| `sin` | `sind` | `asin` | `asind` |
| `cos` | `cosd` | `acos` | `acosd` |
| `tan` | `tand` | `atan` | `atand` |
| `cot` | `cotd` | `acot` | `acotd` |
| `csc` | `cscd` | `acsc` | `acscd` |

Hyperbolic functions are `sinh`, `cosh`, `tanh`, `coth`, `csch` and the inverses `asinh`, `acosh`, `atanh`. For example `sinh(1)` is `1.1752011936`, `cosh(0)` is `1`, `cot(1)` is `0.6420926159` and `cotd(45)` is `1`.

## Rounding

| You type | Answer |
|---|---|
| `round(3.7)` | `4` |
| `round(2.5)` | `3` (halves go away from zero: `round(-2.5)` is `-3`) |
| `floor(-3.2)` | `-4` |
| `int(3.9)`, `trunc(3.9)`, `truncate(3.9)` | `3` (cut toward zero) |
| `ceil(3.2)`, `ceiling(3.2)` | `4` |
| `abs(-5)` | `5` |
| `3.14159 to 2 dp` | `3.14` |
| `round 2.567 to 1 dp` | `2.6` |
| `3.14159 to 2 decimal places` | `3.14` |
| `1234.5678 rounded` | `1,235` |
| `2.5 rounded up` | `3` |
| `17 to nearest 5` | `15` |
| `17 rounded up to nearest 5` | `20` |
| `22 rounded down to nearest 5` | `20` |
| `round 21 up to nearest 5` | `25` |
| `1234 to nearest hundred` | `1,200` |

`to nearest` takes a number, or `ten`, `hundred`, `thousand` or `million`. Rounding to a fraction of an inch (`to nearest 16th`) shows the answer as a fraction. How an answer is shown, in decimals, a fraction or scientific form, is in [Answers and formats](09-answers-and-formats.html).

## Counting and number theory

| You type | Answer |
|---|---|
| `fact(6)`, `6!`, `6 factorial` | `720` |
| `(2 + 3)!` | `120` |
| `gcd(20, 30)`, `gcd of 20 and 30`, `gcd 12, 18` | `10`, `10`, `6` |
| `lcm(4, 6)`, `lcm 4, 6, 10` | `12`, `60` |
| `10 mod 3`, `10 % 3` | `1` |
| `-7 mod 3` | `2` (the result is never negative) |
| `remainder of 21 divided by 5` | `1` |
| `is 59 prime` | `true` |
| `10 permutation 3` | `720` |
| `3 permutations of 10` | `720` |
| `25 combinations 3` | `2,300` |
| `3 combinations of 5` | `10` |

`fact`, `!` and `factorial` need a whole number of zero or more. `gcd` and `lcm` take any number of whole numbers. Very large factorials show in the short notation of your Numbers settings.

## Statistics

Each of these takes a list in brackets or in words, and works on money and units as well as plain numbers.

| You type | Answer |
|---|---|
| `sum(1, 2, 3, 4)` | `10` |
| `sum of 1, 2, 3` | `6` |
| `average(2, 4, 9)`, `avg(4, 8)` | `5`, `6` |
| `average of 1, 2 and 3` | `2` |
| `median(1, 9, 3, 4)` | `3.5` |
| `count(5, 6, 7)` | `3` |
| `max(3, 9, 4)`, `min(3, 9, 4)` | `9`, `3` |
| `max 50, 20, 10` | `50` |
| `larger of 3 and 8` | `8` |
| `smaller of 3 and 8` | `3` |
| `average of 10 m and 60 m` | `35 m` |
| `sum($10, $20)` | `$30.00` |
| `max(5 km, 3000 m)` | `5 km` |
| `stddev(2, 4, 4, 4, 5, 5, 7, 9)` | `2.1380899353` (sample standard deviation) |
| `standard deviation of 2, 4, 4, 4, 5, 5, 7, 9` | `2.1380899353` |

`mean` and `total` are other names for `average` and `sum`, and `maximum` and `minimum` for `max` and `min`. The same words work with a tag on a whole sheet: see [Variables and totals](06-variables-and-totals.html#tags).

Other list helpers:

| You type | Answer |
|---|---|
| `clamp(26, 5, 25)` | `25` |
| `clamp 26 between 5 and 25` | `25` |
| `clamp 4 from 5 to 25` | `5` |
| `midpoint(10, 20)` | `15` |
| `midpoint between 10 and 20` | `15` |
| `halfway between 10 and 20` | `15` |
| `half of 50`, `half(50)` | `25` |
| `random number between 1 and 6` | A whole number from 1 to 6, new each time the sheet is worked out. |

## Logic and bits

Comparisons answer `true` or `false`:

| You type | Answer |
|---|---|
| `5 > 3` | `true` |
| `3 <= 2` | `false` |
| `5 == 5`, `5 != 5` | `true`, `false` |
| `(5 > 3) and (2 > 1)` | `true` |
| `true or false` | `true` |
| `true xor true` | `false` |
| `not true`, `!true` | `false` |
| `true && false` | `false` |

On whole numbers of zero or more, `and`, `or`, `xor`, `&` and the bar `|` work on the bits, and `<<` and `>>` shift them:

```
12 & 10        (8)
12 and 10      (8)
12 | 3         (15)
12 or 3        (15)
12 xor 10      (6)
1 << 4         (16)
256 >> 2       (64)
0xFF & 0x0F    (0xF)
```

A negative number in a bit operation is an error.

Numbers can be written in other bases as `0xFF` (hex), `0b101` (binary) and `0o17` (octal), and shown in them with `as`:

| You type | Answer |
|---|---|
| `255 as hex` | `0xFF` |
| `99 in binary` | `0b1100011` |
| `0b1000101 to octal` | `0o105` |
| `0x2D as base 2` | `0b101101` |
| `255 in base 8` | `0o377` |

A line can also choose between answers with `if`: `if 5 > 3 then 10 else 20` is `10`. See [Variables and totals](06-variables-and-totals.html#conditions).

## Interest and growth

Money math is exact, so the cents agree with a calculator you can check by hand. Write the amount, the time and the rate. Say how often interest compounds with `compounding monthly` (or `compounded quarterly`); without it, interest compounds once a year. The words you can use are `annually`, `yearly`, `semiannually`, `quarterly`, `monthly`, `weekly` and `daily`.

| You type | Answer |
|---|---|
| `$1,000 after 3 years at 7%` | `$1,225.04` |
| `$1,000 for 3 years at 7%` | `$1,225.04` |
| `$1,000 after 3 years at 7% compounding monthly` | `$1,232.93` |
| `$1000 after 10 years at 5% compounded quarterly` | `$1,643.62` |
| `interest on $1,000 for 3 years at 7%` | `$225.04` |
| `interest on $1,000 after 3 years at 7% compounding monthly` | `$232.93` |
| `present value of $10,000 after 5 years at 6%` | `$7,472.58` |
| `present value of $10,000 for 5 years at 6% compounding monthly` | `$7,413.72` |
| `20k after 6 months at 10% per month` | `35,431.22` |

Interest can also be spread over small periods with `hourly`, `daily`, `weekly`, `monthly` or `yearly`:

| You type | Answer |
|---|---|
| `daily interest on $1,000 for 3 years at 4%` | `$0.11` |
| `monthly interest on $1,000 for 3 years at 4% compounding quarterly` | `$3.52` |

Plain multiplication works for a one-off check: `$1,000 * 1.05 ^ 10` is `$1,628.89`.

## Loans

Say what you borrow, for how long and at what rate with `over` and `at`. Start the line with the period you want the answer in: `monthly`, `annual`, `daily` or `total`, then `payment` (or `repayment`), `interest` or `interest paid`.

| You type | Answer |
|---|---|
| `monthly payment on $10,000 over 6 years at 6%` | `$165.73` |
| `total repayment on $10,000 over 6 years at 6%` | `$11,932.48` |
| `annual repayment on $10,000 over 6 years at 6%` | `$1,988.75` |
| `daily repayment on $10,000 over 6 years at 6%` | `$5.45` |
| `total interest on $10,000 over 6 years at 6%` | `$1,932.48` |
| `monthly interest paid on $10,000 over 6 years at 6%` | `$26.84` |
| `monthly payment on $250,000 over 30 years at 6.5%` | `$1,580.17` |

The payment is the standard level monthly payment. The total is the payment times the number of months, and the annual and daily figures spread that total over the years or days.

## Returns and targets

| You type | Answer |
|---|---|
| `$500 invested $1,500 returned` | `2x` (the gain as a multiple of the stake) |
| `annual return on $1,000 invested $2,000 returned after 5 years` | `14.8698354997%` |
| `investment required for $10k/month at 5%` | `$2.4M` |
| `time from $20k to $40k at $2k every month` | `10 months` |
| `time from 20k to 100k at 10% growth per month` | `16.8863170308 months` |
| `growth per day from 10k to 15k over 1 week` | `5.9634022667%` |

Add `in months` (or any unit of time) to the end of a `time from` line to choose the unit.

## Working at a rate

| You type | Answer |
|---|---|
| `30 hours at $30/hour` | `$900.00` |
| `$500 at $20/hour` | `25 hours` |
