//! Cron expression parser.
//! Supports: minute, hour, day-of-month, month, day-of-week
//! Syntax: * */2 1-15 * MON-FRI

use crate::error::SchedulerError;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct CronSchedule {
    pub minutes: HashSet<u8>,
    pub hours: HashSet<u8>,
    pub days_of_month: HashSet<u8>,
    pub months: HashSet<u8>,
    pub days_of_week: HashSet<u8>,
}

impl CronSchedule {
    pub fn parse(expr: &str) -> Result<Self, SchedulerError> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(SchedulerError::InvalidCron(
                "Expected 5 fields".to_string(),
            ));
        }

        Ok(Self {
            minutes: parse_field(parts[0], 0, 59)?,
            hours: parse_field(parts[1], 0, 23)?,
            days_of_month: parse_field(parts[2], 1, 31)?,
            months: parse_field(parts[3], 1, 12)?,
            days_of_week: parse_dow(parts[4])?,
        })
    }

    pub fn matches(&self, minute: u8, hour: u8, day: u8, month: u8, dow: u8) -> bool {
        self.minutes.contains(&minute)
            && self.hours.contains(&hour)
            && self.days_of_month.contains(&day)
            && self.months.contains(&month)
            && self.days_of_week.contains(&dow)
    }
}

fn parse_field(field: &str, min: u8, max: u8) -> Result<HashSet<u8>, SchedulerError> {
    let mut set = HashSet::new();

    for part in field.split(',') {
        if part == "*" {
            for v in min..=max {
                set.insert(v);
            }
        } else if part.starts_with("*/") {
            let step: u8 = part[2..]
                .parse()
                .map_err(|_| SchedulerError::InvalidCron(part.to_string()))?;
            for v in (min..=max).step_by(step as usize) {
                set.insert(v);
            }
        } else if part.contains('-') {
            let bounds: Vec<&str> = part.split('-').collect();
            let start: u8 = bounds[0]
                .parse()
                .map_err(|_| SchedulerError::InvalidCron(part.to_string()))?;
            let end: u8 = bounds[1]
                .parse()
                .map_err(|_| SchedulerError::InvalidCron(part.to_string()))?;
            for v in start..=end {
                set.insert(v);
            }
        } else {
            let val: u8 = part
                .parse()
                .map_err(|_| SchedulerError::InvalidCron(part.to_string()))?;
            if val < min || val > max {
                return Err(SchedulerError::InvalidCron(part.to_string()));
            }
            set.insert(val);
        }
    }

    Ok(set)
}

fn parse_dow(field: &str) -> Result<HashSet<u8>, SchedulerError> {
    let dow_map = [
        ("SUN", 0), ("MON", 1), ("TUE", 2), ("WED", 3), ("THU", 4),
        ("FRI", 5), ("SAT", 6),
    ];

    if field == "*" {
        return parse_field(field, 0, 6);
    }

    let mut result = HashSet::new();
    for part in field.split(',') {
        if part.contains('-') {
            // Handle ranges like MON-FRI
            let bounds: Vec<&str> = part.split('-').collect();
            if bounds.len() == 2 {
                let start_name = bounds[0].trim().to_uppercase();
                let end_name = bounds[1].trim().to_uppercase();

                let start_num = dow_map.iter()
                    .find(|(n, _)| *n == start_name)
                    .ok_or_else(|| SchedulerError::InvalidCron(part.to_string()))?;
                let end_num = dow_map.iter()
                    .find(|(n, _)| *n == end_name)
                    .ok_or_else(|| SchedulerError::InvalidCron(part.to_string()))?;

                for num in start_num.1..=end_num.1 {
                    result.insert(num);
                }
            }
        } else {
            // Single day name or number
            let upper = part.trim().to_uppercase();
            let found = dow_map.iter()
                .find(|(n, _)| *n == upper);

            if let Some((_, num)) = found {
                result.insert(*num);
            } else if let Ok(val) = part.parse::<u8>() {
                if val <= 6 {
                    result.insert(val);
                }
            }
        }
    }

    if result.is_empty() {
        return Err(SchedulerError::InvalidCron(field.to_string()));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_star() {
        let schedule = CronSchedule::parse("* * * * *").unwrap();
        assert!(schedule.matches(0, 0, 1, 1, 0));
        assert!(schedule.matches(59, 23, 31, 12, 6));
    }

    #[test]
    fn test_parse_specific_minute() {
        let schedule = CronSchedule::parse("30 * * * *").unwrap();
        assert!(schedule.matches(30, 12, 15, 6, 1));
        assert!(!schedule.matches(31, 12, 15, 6, 1));
    }

    #[test]
    fn test_parse_step() {
        let schedule = CronSchedule::parse("*/15 * * * *").unwrap();
        assert!(schedule.matches(0, 0, 1, 1, 0));
        assert!(schedule.matches(15, 0, 1, 1, 0));
        assert!(!schedule.matches(7, 0, 1, 1, 0));
    }

    #[test]
    fn test_parse_range() {
        let schedule = CronSchedule::parse("0 9-17 * * *").unwrap();
        assert!(schedule.matches(0, 9, 15, 6, 1));
        assert!(schedule.matches(0, 17, 15, 6, 1));
        assert!(!schedule.matches(0, 8, 15, 6, 1));
    }

    #[test]
    fn test_parse_dow_names() {
        let schedule = CronSchedule::parse("0 9 * * MON-FRI").unwrap();
        // MON=1
        assert!(schedule.matches(0, 9, 15, 6, 1));
        // SAT=6
        assert!(!schedule.matches(0, 9, 15, 6, 6));
    }

    #[test]
    fn test_invalid_expression() {
        let result = CronSchedule::parse("* *");
        assert!(result.is_err());
    }
}
