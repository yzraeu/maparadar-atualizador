use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct AlertType {
    pub code: u8,
    pub label: &'static str,
    pub icon: &'static str,
    pub default: bool,
}

pub const ALERT_TYPES: &[AlertType] = &[
    AlertType { code: 1, label: "Radar Fixo", icon: "fixed_110km", default: true },
    AlertType { code: 2, label: "Radar Móvel", icon: "mobile_110km", default: true },
    AlertType { code: 4, label: "Semáforo c/ Câmera", icon: "traffic_camera", default: true },
    AlertType { code: 5, label: "Semáforo c/ Radar", icon: "traffic_light_80km", default: true },
    AlertType { code: 6, label: "Polícia Rodoviária", icon: "highway_patrol", default: true },
    AlertType { code: 7, label: "Pedágio", icon: "toll", default: false },
    AlertType { code: 9, label: "Lombada", icon: "speed_bump", default: false },
];

#[allow(dead_code)]
pub fn default_selected() -> Vec<u8> {
    ALERT_TYPES.iter().filter(|a| a.default).map(|a| a.code).collect()
}

#[allow(dead_code)]
pub fn radar_types_string(selected: &[u8]) -> String {
    selected.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_selected_returns_the_five_default_types() {
        assert_eq!(default_selected(), vec![1, 2, 4, 5, 6]);
    }

    #[test]
    fn radar_types_string_joins_with_comma() {
        assert_eq!(radar_types_string(&[1, 2, 4]), "1,2,4");
        assert_eq!(radar_types_string(&[]), "");
    }

    #[test]
    fn catalog_matches_site_codes_and_defaults() {
        let codes: Vec<u8> = ALERT_TYPES.iter().map(|a| a.code).collect();
        assert_eq!(codes, vec![1, 2, 4, 5, 6, 7, 9]);
        let default_off: Vec<u8> = ALERT_TYPES.iter().filter(|a| !a.default).map(|a| a.code).collect();
        assert_eq!(default_off, vec![7, 9]);
    }
}
