//! Windows GPU Engine instances contain a LUID, physical adapter and engine.
//! Sum processes on each engine, then select the busiest engine per adapter.
use std::collections::HashMap;

pub(super) fn aggregate(
    samples: impl IntoIterator<Item = (String, Option<f64>)>,
) -> HashMap<String, f32> {
    let mut engines: HashMap<(String, u32, u32), Option<f64>> = HashMap::new();
    for (name, value) in samples {
        let Some((_, suffix)) = name.split_once("_luid_") else {
            continue;
        };
        let parts: Vec<_> = suffix.split('_').collect();
        if parts.len() < 6 || parts[2] != "phys" || parts[4] != "eng" {
            continue;
        }
        let (Ok(high), Ok(low), Ok(physical), Ok(engine)) = (
            u32::from_str_radix(parts[0].trim_start_matches("0x"), 16),
            u32::from_str_radix(parts[1].trim_start_matches("0x"), 16),
            parts[3].parse::<u32>(),
            parts[5].parse::<u32>(),
        ) else {
            continue;
        };
        let key = (format!("dxcore:{high:08x}{low:08x}"), physical, engine);
        let sum = engines.entry(key).or_insert(Some(0.0));
        *sum = match (*sum, value) {
            (Some(a), Some(b)) if b.is_finite() && b >= 0.0 => Some(a + b),
            _ => None,
        };
    }
    let mut adapters: HashMap<String, Option<f64>> = HashMap::new();
    for ((id, _, _), sum) in engines {
        let maximum = adapters.entry(id).or_insert(Some(0.0));
        *maximum = match (*maximum, sum) {
            (Some(a), Some(b)) => Some(a.max(b)),
            _ => None,
        };
    }
    adapters
        .into_iter()
        .filter_map(|(id, load)| Some((id, load?.clamp(0.0, 100.0) as f32)))
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn row(pid: u32, gpu: u32, engine: u32, value: Option<f64>) -> (String, Option<f64>) {
        (
            format!("pid_{pid}_luid_0x00000000_0x{gpu:08x}_phys_0_eng_{engine}_engtype_3D"),
            value,
        )
    }
    #[test]
    fn sums_processes_without_summing_independent_engines_or_gpus() {
        let result = aggregate([
            row(1, 1, 0, Some(25.0)),
            row(2, 1, 0, Some(30.0)),
            row(1, 1, 1, Some(80.0)),
            row(1, 2, 0, Some(0.0)),
        ]);
        assert_eq!(result["dxcore:0000000000000001"], 80.0);
        assert_eq!(result["dxcore:0000000000000002"], 0.0);
    }
    #[test]
    fn invalid_counter_invalidates_adapter_and_missing_is_not_idle() {
        let result = aggregate([
            row(1, 1, 0, Some(25.0)),
            row(2, 1, 1, None),
            row(3, 2, 0, Some(f64::NAN)),
        ]);
        assert!(result.is_empty());
        assert!(aggregate([]).is_empty());
    }
}
