use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{space0, space1},
    combinator::{opt, recognize},
    multi::many1,
    number::complete::double,
    sequence::{separated_pair, terminated},
    Finish, IResult, Parser,
};
use std::time::Duration;

use crate::units::Unit;

// Check if a given f64 numbers fits in u64
fn parse_decimal(value: f64) -> Option<f64> {
    if value >= 0.0 && value <= u64::MAX as f64 {
        Some(value)
    } else {
        None
    }
}

// Convert parsed units to seconds and nanoseconds
fn convert_to_duration(value: f64, unit: Unit) -> Duration {
    let total_seconds = unit.to_second(value);
    let seconds = total_seconds.floor() as u64;
    let nanos = ((total_seconds - total_seconds.floor()) * 1e9).round() as u32;

    Duration::new(seconds, nanos)
}

// Parse a unit name
fn unit(input: &str) -> IResult<&str, Unit> {
    let nanosecond = alt((tag("nanos"), tag("nsec"), tag("ns"))).map(|_| Unit::Nanos);
    let microsecond = alt((tag("micros"), tag("usec"), tag("us"))).map(|_| Unit::Micros);
    let millisecond = alt((tag("millis"), tag("msec"), tag("ms"))).map(|_| Unit::Millis);
    let seconds = alt((
        tag("seconds"),
        tag("second"),
        tag("secs"),
        tag("sec"),
        tag("s"),
    ))
    .map(|_| Unit::Seconds);
    let minutes = alt((
        tag("minutes"),
        tag("minute"),
        tag("mins"),
        tag("min"),
        tag("m"),
    ))
    .map(|_| Unit::Minutes);
    let hours = alt((
        tag("hours"),
        tag("hour"),
        tag("hrs"),
        tag("hr"),
        tag("h"),
        tag("H"),
    ))
    .map(|_| Unit::Hours);
    let days = alt((
        tag("days"),
        tag("day"),
        tag("dys"),
        tag("dy"),
        tag("d"),
        tag("D"),
    ))
    .map(|_| Unit::Days);
    let weeks = alt((
        tag("weeks"),
        tag("week"),
        tag("wks"),
        tag("wk"),
        tag("w"),
        tag("W"),
    ))
    .map(|_| Unit::Weeks);
    let months = alt((
        tag("months"),
        tag("month"),
        tag("mths"),
        tag("mth"),
        tag("M"),
    ))
    .map(|_| Unit::Months);
    let years = alt((
        tag("years"),
        tag("year"),
        tag("yrs"),
        tag("yr"),
        tag("y"),
        tag("Y"),
    ))
    .map(|_| Unit::Years);

    alt((
        months,
        days,
        weeks,
        years,
        nanosecond,
        microsecond,
        millisecond,
        seconds,
        minutes,
        hours,
    ))
    .parse(input)
}

fn number(input: &str) -> IResult<&str, f64> {
    double.map_opt(parse_decimal).parse(input)
}

// Parse a float followed by a unit
fn time_span(input: &str) -> IResult<&str, Duration> {
    let number_input = separated_pair(number, opt(space0), unit);
    let and_with_spaces = recognize((opt(space1), tag("and"), opt(space1)));
    let duration_sep = alt((and_with_spaces, space1));

    let (input, (value, unit)) = terminated(number_input, opt(duration_sep)).parse(input)?;
    Ok((input, convert_to_duration(value, unit)))
}

/// Error parsing human-friendly duration
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Input is empty.
    EmptyInput,
    /// Failed to fully parse given input.
    ParseFailed(String),
    /// Error parsing input with nom.
    Nom(nom::error::Error<String>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EmptyInput => write!(f, "input is empty"),
            Error::ParseFailed(left_over) => write!(f, "parsing duration failed at: {left_over}"),
            Error::Nom(error) => write!(f, "parse duration error: {error}"),
        }
    }
}

impl From<nom::error::Error<String>> for Error {
    fn from(value: nom::error::Error<String>) -> Self {
        Self::Nom(value)
    }
}

/// Parse duration object `1hour 12min 5s`
///
/// The duration object is a concatenation of time spans. Where each time
/// span is an integer number and a suffix. Supported suffixes:
///
/// * `nanos`, `nsec`, `ns` -- nanoseconds
/// * `micros`, `usec`, `us` -- microseconds
/// * `millis`, `msec`, `ms` -- milliseconds
/// * `seconds`, `second`, `secs`, `sec`, `s`
/// * `minutes`, `minute`, `mins`, `min`, `m`
/// * `hours`, `hour`, `hrs`, `hr`, `h`, `H`
/// * `days`, `day`, `dys`, `dy`, `d`, `D`
/// * `weeks`, `week`, `wks`, `wk`, `w`, `W`
/// * `months`, `month`, `mths`, `mth`, `M` -- defined as 30.44 days
/// * `years`, `year`, `yrs`, `yr`, `y`, `Y` -- defined as 365.25 days
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use humantime::parse_duration;
///
/// assert_eq!(parse_duration("2h 37min"), Ok(Duration::new(9420, 0)));
/// assert_eq!(parse_duration("32ms"), Ok(Duration::new(0, 32_000_000)));
/// assert_eq!(parse_duration("2 minutes"), Ok(Duration::new(120, 0)));
/// assert_eq!(parse_duration("2 minutes and 30 seconds"), Ok(Duration::new(150, 0)));
/// assert_eq!(parse_duration("2hrs2mins"), Ok(Duration::new(7320, 0)));
/// assert_eq!(parse_duration("2days and 2mins"), Ok(Duration::new(172_920, 0)));
/// assert_eq!(parse_duration(".5mins"), Ok(Duration::new(30, 0)));
/// assert_eq!(parse_duration("1.5 mins"), Ok(Duration::new(90, 0)));
/// assert_eq!(parse_duration("0.1 days"), Ok(Duration::new(8640, 0)));
/// assert_eq!(parse_duration("11e-1 days"), Ok(Duration::new(95_040, 0)));
/// ```
pub fn parse_duration(input: &str) -> Result<Duration, Error> {
    let input = input.trim();
    if input.is_empty() {
        return Err(Error::EmptyInput);
    }

    if input == "0" {
        return Ok(Duration::new(0, 0));
    }

    let (input, durations) = many1(time_span)
        .parse(input)
        .map_err(|e| e.to_owned())
        .finish()?;

    let input = input.trim();
    if !input.trim().is_empty() {
        return Err(Error::ParseFailed(input.to_owned()));
    }

    let total_duration = durations
        .into_iter()
        .fold(Duration::new(0, 0), |acc, duration| acc + duration);
    Ok(total_duration)
}

#[cfg(test)]
mod test {
    use crate::format_duration;

    use super::{parse_duration, Error};
    use std::time::Duration;

    macro_rules! assert_parse_duration_ok {
        ($input:expr, $secs:expr, $nanos:expr) => {
            assert_eq!(parse_duration($input), Ok(Duration::new($secs, $nanos)));
        };
    }

    macro_rules! assert_parse_duration_err {
        ($input:expr) => {
            assert_eq!(
                parse_duration($input),
                Err(Error::Nom(nom::error::Error::new(
                    $input.to_owned(),
                    nom::error::ErrorKind::MapOpt
                )))
            );
        };
    }

    #[test]
    fn test_nanosecond() {
        assert_parse_duration_ok!("1nanos", 0, 1);
        assert_parse_duration_ok!("1 nanos", 0, 1);

        assert_parse_duration_ok!("2nsec", 0, 2);
        assert_parse_duration_ok!("2 nsec", 0, 2);

        assert_parse_duration_ok!("3ns", 0, 3);
        assert_parse_duration_ok!("3 ns", 0, 3);
    }

    #[test]
    fn test_microsecond() {
        assert_parse_duration_ok!("1micros", 0, 1000);
        assert_parse_duration_ok!("1 micros", 0, 1000);

        assert_parse_duration_ok!("2usec", 0, 2000);
        assert_parse_duration_ok!("2 usec", 0, 2000);

        assert_parse_duration_ok!("3us", 0, 3000);
        assert_parse_duration_ok!("3 us", 0, 3000);
    }

    #[test]
    fn test_millisecond() {
        assert_parse_duration_ok!("1millis", 0, 1_000_000);
        assert_parse_duration_ok!("1 millis", 0, 1_000_000);

        assert_parse_duration_ok!("2msec", 0, 2_000_000);
        assert_parse_duration_ok!("2 msec", 0, 2_000_000);

        assert_parse_duration_ok!("3ms", 0, 3_000_000);
        assert_parse_duration_ok!("3 ms", 0, 3_000_000);
    }

    #[test]
    fn test_seconds() {
        assert_parse_duration_ok!("1seconds", 1, 0);
        assert_parse_duration_ok!("1 seconds", 1, 0);

        assert_parse_duration_ok!("2second", 2, 0);
        assert_parse_duration_ok!("2 second", 2, 0);

        assert_parse_duration_ok!("3secs", 3, 0);
        assert_parse_duration_ok!("3 secs", 3, 0);

        assert_parse_duration_ok!("4sec", 4, 0);
        assert_parse_duration_ok!("4 sec", 4, 0);

        assert_parse_duration_ok!("5s", 5, 0);
        assert_parse_duration_ok!("5 s", 5, 0);
    }

    #[test]
    fn test_minutes() {
        assert_parse_duration_ok!("1minutes", 1 * 60, 0);
        assert_parse_duration_ok!("1 minutes", 1 * 60, 0);

        assert_parse_duration_ok!("2minute", 2 * 60, 0);
        assert_parse_duration_ok!("2 minute", 2 * 60, 0);

        assert_parse_duration_ok!("3mins", 3 * 60, 0);
        assert_parse_duration_ok!("3 mins", 3 * 60, 0);

        assert_parse_duration_ok!("4min", 4 * 60, 0);
        assert_parse_duration_ok!("4 min", 4 * 60, 0);

        assert_parse_duration_ok!("5m", 5 * 60, 0);
        assert_parse_duration_ok!("5 m", 5 * 60, 0);
    }

    #[test]
    fn test_hours() {
        assert_parse_duration_ok!("1hours", 1 * 3600, 0);
        assert_parse_duration_ok!("1 hours", 1 * 3600, 0);

        assert_parse_duration_ok!("2hour", 2 * 3600, 0);
        assert_parse_duration_ok!("2 hour", 2 * 3600, 0);

        assert_parse_duration_ok!("3hrs", 3 * 3600, 0);
        assert_parse_duration_ok!("3 hrs", 3 * 3600, 0);

        assert_parse_duration_ok!("4hr", 4 * 3600, 0);
        assert_parse_duration_ok!("4 hr", 4 * 3600, 0);

        assert_parse_duration_ok!("5h", 5 * 3600, 0);
        assert_parse_duration_ok!("5 h", 5 * 3600, 0);

        assert_parse_duration_ok!("5H", 5 * 3600, 0);
        assert_parse_duration_ok!("5 H", 5 * 3600, 0);
    }

    #[test]
    fn test_days() {
        assert_parse_duration_ok!("1days", 1 * 86400, 0);
        assert_parse_duration_ok!("1 days", 1 * 86400, 0);

        assert_parse_duration_ok!("2day", 2 * 86400, 0);
        assert_parse_duration_ok!("2 day", 2 * 86400, 0);

        assert_parse_duration_ok!("3dys", 3 * 86400, 0);
        assert_parse_duration_ok!("3 dys", 3 * 86400, 0);

        assert_parse_duration_ok!("4dy", 4 * 86400, 0);
        assert_parse_duration_ok!("4 dy", 4 * 86400, 0);

        assert_parse_duration_ok!("5d", 5 * 86400, 0);
        assert_parse_duration_ok!("5 d", 5 * 86400, 0);

        assert_parse_duration_ok!("5D", 5 * 86400, 0);
        assert_parse_duration_ok!("5 D", 5 * 86400, 0);
    }

    #[test]
    fn test_weeks() {
        assert_parse_duration_ok!("1weeks", 1 * 604_800, 0);
        assert_parse_duration_ok!("1 weeks", 1 * 604_800, 0);

        assert_parse_duration_ok!("2week", 2 * 604_800, 0);
        assert_parse_duration_ok!("2 week", 2 * 604_800, 0);

        assert_parse_duration_ok!("3wks", 3 * 604_800, 0);
        assert_parse_duration_ok!("3 wks", 3 * 604_800, 0);

        assert_parse_duration_ok!("4wk", 4 * 604_800, 0);
        assert_parse_duration_ok!("4 wk", 4 * 604_800, 0);

        assert_parse_duration_ok!("5w", 5 * 604_800, 0);
        assert_parse_duration_ok!("5 w", 5 * 604_800, 0);

        assert_parse_duration_ok!("5W", 5 * 604_800, 0);
        assert_parse_duration_ok!("5 W", 5 * 604_800, 0);
    }

    #[test]
    fn test_months() {
        assert_parse_duration_ok!("1months", 1 * 2_630_016, 0);
        assert_parse_duration_ok!("1 months", 1 * 2_630_016, 0);

        assert_parse_duration_ok!("2month", 2 * 2_630_016, 0);
        assert_parse_duration_ok!("2 month", 2 * 2_630_016, 0);

        assert_parse_duration_ok!("3mths", 3 * 2_630_016, 0);
        assert_parse_duration_ok!("3 mths", 3 * 2_630_016, 0);

        assert_parse_duration_ok!("4mth", 4 * 2_630_016, 0);
        assert_parse_duration_ok!("4 mth", 4 * 2_630_016, 0);

        assert_parse_duration_ok!("5M", 5 * 2_630_016, 0);
        assert_parse_duration_ok!("5 M", 5 * 2_630_016, 0);
    }

    #[test]
    fn test_years() {
        assert_parse_duration_ok!("1years", 1 * 31_557_600, 0);
        assert_parse_duration_ok!("1 years", 1 * 31_557_600, 0);

        assert_parse_duration_ok!("2year", 2 * 31_557_600, 0);
        assert_parse_duration_ok!("2 year", 2 * 31_557_600, 0);

        assert_parse_duration_ok!("3yrs", 3 * 31_557_600, 0);
        assert_parse_duration_ok!("3 yrs", 3 * 31_557_600, 0);

        assert_parse_duration_ok!("4yr", 4 * 31_557_600, 0);
        assert_parse_duration_ok!("4 yr", 4 * 31_557_600, 0);

        assert_parse_duration_ok!("5y", 5 * 31_557_600, 0);
        assert_parse_duration_ok!("5 y", 5 * 31_557_600, 0);

        assert_parse_duration_ok!("5Y", 5 * 31_557_600, 0);
        assert_parse_duration_ok!("5 Y", 5 * 31_557_600, 0);
    }

    #[test]
    fn test_fractions() {
        assert_parse_duration_ok!(".5m", 30, 0);
        assert_parse_duration_ok!("1.5m", 90, 0);
        assert_parse_duration_ok!("3.44d", 297_216, 0);
        assert_parse_duration_ok!("0.0001 days", 8, 640_000_000);
        assert_parse_duration_ok!("11e-1 days", 95_040, 0);
        assert_parse_duration_ok!("11.2e-1 days", 96_768, 0);
    }

    #[test]
    fn allow_0_with_no_unit() {
        assert_parse_duration_ok!("0", 0, 0);
    }

    #[test]
    fn test_combo() {
        assert_parse_duration_ok!("20 min 17 nsec", 1200, 17);
        assert_parse_duration_ok!("20min17nsec", 1200, 17);
        assert_parse_duration_ok!("2h 15m", 8100, 0);
        assert_parse_duration_ok!("2hand15m", 8100, 0);
        assert_parse_duration_ok!("2h and 15m", 8100, 0);
        assert_parse_duration_ok!("2hand 15m", 8100, 0);
    }

    #[test]
    fn test_overlow() {
        assert_parse_duration_err!("100000000000000000000ns");
        assert_parse_duration_err!("100000000000000000000us");
        assert_parse_duration_err!("100000000000000000000ms");
        assert_parse_duration_err!("100000000000000000000s");
        assert_parse_duration_err!("100000000000000000000m");
        assert_parse_duration_err!("100000000000000000000h");
        assert_parse_duration_err!("100000000000000000000d");
        assert_parse_duration_err!("100000000000000000000w");
        assert_parse_duration_err!("100000000000000000000M");
        assert_parse_duration_err!("100000000000000000000Y");
    }

    #[test]
    fn all_86400_seconds() {
        for second in 0..86400 {
            let d = Duration::new(second, 0);
            assert_eq!(d, parse_duration(&format_duration(d).to_string()).unwrap());
        }
    }

    #[test]
    fn random_second() {
        use rand::Rng;
        for _ in 0..10000 {
            let sec = rand::rng().random_range(0..253_370_764_800);
            let d = Duration::new(sec, 0);
            assert_eq!(d, parse_duration(&format_duration(d).to_string()).unwrap());
        }
    }

    // #[test]
    // fn random_any() {
    //     use rand::Rng;
    //     for _ in 0..10000 {
    //         let sec = rand::rng().random_range(0..253_370_764_800);
    //         let nanos = rand::rng().random_range(0..1_000_000_000);
    //         let d = Duration::new(sec, nanos);
    //         assert_eq!(d, parse_duration(&format_duration(d).to_string()).unwrap());
    //     }
    // }

    // #[test]
    // fn test_nice_error_message() {
    //     assert_eq!(
    //         parse_duration("123").unwrap_err().to_string(),
    //         "time unit needed, for example 123sec or 123ms"
    //     );
    //     assert_eq!(
    //         parse_duration("10 months 1").unwrap_err().to_string(),
    //         "time unit needed, for example 1sec or 1ms"
    //     );
    //     assert_eq!(
    //         parse_duration("10nights").unwrap_err().to_string(),
    //         "unknown time unit \"nights\", supported units: \
    //         ns, us, ms, sec, min, hours, days, weeks, months, \
    //         years (and few variations)"
    //     );
    // }

    // #[test]
    // fn test_error_cases() {
    //     assert_eq!(
    //         parse_duration("\0").unwrap_err().to_string(),
    //         "expected number at 0"
    //     );
    //     assert_eq!(
    //         parse_duration("\r").unwrap_err().to_string(),
    //         "value was empty"
    //     );
    //     assert_eq!(
    //         parse_duration("1~").unwrap_err().to_string(),
    //         "invalid character at 1"
    //     );
    //     assert_eq!(
    //         parse_duration("1Nå").unwrap_err().to_string(),
    //         "invalid character at 2"
    //     );
    //     assert_eq!(parse_duration("222nsec221nanosmsec7s5msec572s").unwrap_err().to_string(),
    //                "unknown time unit \"nanosmsec\", supported units: ns, us, ms, sec, min, hours, days, weeks, months, years (and few variations)");
    // }
}
