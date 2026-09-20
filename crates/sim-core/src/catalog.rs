//! S11 — typed kernel catalog (demo taxa from docs/data/kernel_catalog.csv).
//!
//! Occupant scalars come from a row keyed by `taxon_id`. Missing numeric/
//! string fields are `None`; `root_mask` uses form defaults when depth is null.
//! S12: climate Option fields drive World envelope vetoes.

use crate::{Occupant, N_LAYERS};
use std::collections::HashMap;

/// Growth form used for root-mask defaults when `root_depth_m` is absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Herb,
    Shrub,
    Tree,
}

impl Form {
    fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "herb" => Some(Form::Herb),
            "shrub" => Some(Form::Shrub),
            "tree" => Some(Form::Tree),
            _ => None,
        }
    }

    /// Default root presence mask when depth is unknown. Herb → top only; shrub/tree → both.
    pub fn default_root_mask(self) -> [f64; N_LAYERS] {
        match self {
            Form::Herb => [1.0, 0.0],
            Form::Shrub | Form::Tree => [1.0, 1.0],
        }
    }
}

/// Typed row for one taxon. Climate fields feed the S12 envelope filter.
#[derive(Clone, Debug, PartialEq)]
pub struct OccupantParams {
    pub taxon_id: String,
    pub name_norm: String,
    pub scientific: Option<String>,
    pub demo: bool,
    pub form: Form,
    pub height_m_mature: Option<f64>,
    pub root_depth_m: Option<f64>,
    pub root_habit: Option<String>,
    pub sla_m2_kg: Option<f64>,
    pub t_min_c: Option<f64>,
    pub t_max_c: Option<f64>,
    pub rain_min_mm: Option<f64>,
    pub rain_max_mm: Option<f64>,
    pub texture_ok: Option<String>,
    pub drain_ok: Option<String>,
    pub phenology: Option<String>,
    pub dispersal: Option<String>,
    /// Presence mask derived from `root_depth_m` / form (not normalized weights).
    pub root_mask: [f64; N_LAYERS],
}

impl OccupantParams {
    /// Build a PlantStub occupant: uptake [`P_MAX`], shade 0.25 (PlantStub default).
    pub fn to_plant_stub(&self) -> Occupant {
        let mut occ = Occupant::plant_stub_with_root(self.root_mask);
        occ.taxon_id = Some(self.taxon_id.clone());
        occ
    }
}

/// Map root depth to the locked two-layer presence mask.
///
/// - `None` → form default (herb `[1,0]`; shrub/tree `[1,1]`)
/// - depth ≤ 0.20 → `[1,0]`
/// - else → `[1,1]`
pub fn root_mask_from_depth(root_depth_m: Option<f64>, form: Form) -> [f64; N_LAYERS] {
    match root_depth_m {
        None => form.default_root_mask(),
        Some(d) if d <= 0.20 => [1.0, 0.0],
        Some(_) => [1.0, 1.0],
    }
}

/// Error loading or querying the catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogError {
    Parse(String),
    UnknownTaxon(String),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::Parse(m) => write!(f, "catalog parse: {m}"),
            CatalogError::UnknownTaxon(id) => write!(f, "unknown taxon_id: {id}"),
        }
    }
}

impl std::error::Error for CatalogError {}

/// In-memory catalog keyed by `taxon_id`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Catalog {
    by_id: HashMap<String, OccupantParams>,
}

impl Catalog {
    /// Compile-time embed of `docs/data/kernel_catalog.csv`.
    pub fn load_embedded() -> Result<Self, CatalogError> {
        const CSV: &str = include_str!("../../../docs/data/kernel_catalog.csv");
        Self::from_csv_str(CSV)
    }

    /// Parse a CSV string (header + rows). Empty cells → `None`.
    pub fn from_csv_str(csv: &str) -> Result<Self, CatalogError> {
        let mut lines = csv.lines().filter(|l| !l.trim().is_empty());
        let header = lines
            .next()
            .ok_or_else(|| CatalogError::Parse("empty csv".into()))?;
        let cols = split_csv_line(header);
        let idx = |name: &str| -> Result<usize, CatalogError> {
            cols.iter()
                .position(|c| c.as_str() == name)
                .ok_or_else(|| CatalogError::Parse(format!("missing column {name}")))
        };
        let i_taxon = idx("taxon_id")?;
        let i_name = idx("name_norm")?;
        let i_sci = idx("scientific")?;
        let i_demo = idx("demo")?;
        let i_form = idx("form")?;
        let i_height = idx("height_m_mature")?;
        let i_root = idx("root_depth_m")?;
        let i_habit = idx("root_habit")?;
        let i_sla = idx("sla_m2_kg")?;
        let i_tmin = idx("t_min_c")?;
        let i_tmax = idx("t_max_c")?;
        let i_rmin = idx("rain_min_mm")?;
        let i_rmax = idx("rain_max_mm")?;
        let i_tex = idx("texture_ok")?;
        let i_drain = idx("drain_ok")?;
        let i_phen = idx("phenology")?;
        let i_disp = idx("dispersal")?;

        let mut by_id = HashMap::new();
        for (lineno, line) in lines.enumerate() {
            let row = split_csv_line(line);
            let get = |i: usize| -> &str { row.get(i).map(|s| s.as_str()).unwrap_or("") };
            let taxon_id = get(i_taxon).trim().to_string();
            if taxon_id.is_empty() {
                return Err(CatalogError::Parse(format!(
                    "line {}: empty taxon_id",
                    lineno + 2
                )));
            }
            let form_s = get(i_form).trim();
            let form = Form::parse(form_s).ok_or_else(|| {
                CatalogError::Parse(format!(
                    "line {}: bad form {form_s:?} for {taxon_id}",
                    lineno + 2
                ))
            })?;
            let root_depth_m = parse_opt_f64(get(i_root))?;
            let root_mask = root_mask_from_depth(root_depth_m, form);
            let params = OccupantParams {
                taxon_id: taxon_id.clone(),
                name_norm: get(i_name).trim().to_string(),
                scientific: opt_string(get(i_sci)),
                demo: parse_demo(get(i_demo))?,
                form,
                height_m_mature: parse_opt_f64(get(i_height))?,
                root_depth_m,
                root_habit: opt_string(get(i_habit)),
                sla_m2_kg: parse_opt_f64(get(i_sla))?,
                t_min_c: parse_opt_f64(get(i_tmin))?,
                t_max_c: parse_opt_f64(get(i_tmax))?,
                rain_min_mm: parse_opt_f64(get(i_rmin))?,
                rain_max_mm: parse_opt_f64(get(i_rmax))?,
                texture_ok: opt_string(get(i_tex)),
                drain_ok: opt_string(get(i_drain)),
                phenology: opt_string(get(i_phen)),
                dispersal: opt_string(get(i_disp)),
                root_mask,
            };
            if by_id.insert(taxon_id.clone(), params).is_some() {
                return Err(CatalogError::Parse(format!(
                    "duplicate taxon_id {taxon_id}"
                )));
            }
        }
        Ok(Catalog { by_id })
    }

    /// Look up by `taxon_id`. Unknown id → Err.
    pub fn get(&self, taxon_id: &str) -> Result<&OccupantParams, CatalogError> {
        self.by_id
            .get(taxon_id)
            .ok_or_else(|| CatalogError::UnknownTaxon(taxon_id.to_string()))
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Count of rows with `demo == true`.
    pub fn demo_count(&self) -> usize {
        self.by_id.values().filter(|p| p.demo).count()
    }

    /// Iterate all params (order unspecified).
    pub fn iter(&self) -> impl Iterator<Item = &OccupantParams> {
        self.by_id.values()
    }
}

fn split_csv_line(line: &str) -> Vec<String> {
    // Demo catalog has no quoted commas; keep a minimal splitter.
    line.split(',').map(|s| s.to_string()).collect()
}

fn opt_string(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn parse_opt_f64(s: &str) -> Result<Option<f64>, CatalogError> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>()
        .map(Some)
        .map_err(|e| CatalogError::Parse(format!("bad f64 {t:?}: {e}")))
}

fn parse_demo(s: &str) -> Result<bool, CatalogError> {
    match s.trim() {
        "1" | "true" | "TRUE" => Ok(true),
        "0" | "false" | "FALSE" | "" => Ok(false),
        other => Err(CatalogError::Parse(format!("bad demo flag {other:?}"))),
    }
}

#[cfg(test)]
mod catalog_unit {
    use super::*;

    #[test]
    fn root_mask_thresholds() {
        assert_eq!(root_mask_from_depth(None, Form::Herb), [1.0, 0.0]);
        assert_eq!(root_mask_from_depth(None, Form::Tree), [1.0, 1.0]);
        assert_eq!(root_mask_from_depth(Some(0.20), Form::Herb), [1.0, 0.0]);
        assert_eq!(root_mask_from_depth(Some(0.21), Form::Herb), [1.0, 1.0]);
    }
}
