// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use wayland_present::OutputLayout;

#[derive(Debug, Clone)]
pub struct MonitorTopology {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: i32,
    #[allow(dead_code)]
    pub refresh_rate_hz: u32,
}

#[derive(Debug, Clone)]
pub struct DisplayTopologyMap {
    pub monitors: Vec<MonitorTopology>,
    pub independent_rendering: bool,
}

impl DisplayTopologyMap {
    pub fn build(layouts: &[OutputLayout]) -> Self {
        let independent_rendering = std::env::var("TRANCE_INDEPENDENT_RENDERING")
            .map(|val| val == "1" || val.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let custom_layouts = std::env::var("TRANCE_CUSTOM_LAYOUTS")
            .ok()
            .map(|s| parse_custom_layouts(&s))
            .unwrap_or_default();

        let mut monitors = Vec::new();
        for layout in layouts {
            let mut x = layout.x;
            let mut y = layout.y;
            let mut w = layout.width;
            let mut h = layout.height;
            let mut scale = layout.scale;

            if let Some(custom) = custom_layouts.get(&layout.id) {
                if let Some(cx) = custom.x { x = cx; }
                if let Some(cy) = custom.y { y = cy; }
                if let Some(cw) = custom.w { w = cw; }
                if let Some(ch) = custom.h { h = ch; }
                if let Some(cs) = custom.scale { scale = cs; }
            }

            monitors.push(MonitorTopology {
                id: layout.id,
                x,
                y,
                width: w,
                height: h,
                scale,
                refresh_rate_hz: layout.refresh_rate_hz,
            });
        }

        Self {
            monitors,
            independent_rendering,
        }
    }
}

#[derive(Default)]
struct CustomOverride {
    x: Option<i32>,
    y: Option<i32>,
    w: Option<u32>,
    h: Option<u32>,
    scale: Option<i32>,
}

fn parse_custom_layouts(s: &str) -> HashMap<u32, CustomOverride> {
    let mut map = HashMap::new();
    // format: "id:x,y,w,h,scale;..."
    for entry in s.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let parts: Vec<&str> = entry.split(':').collect();
        if parts.len() == 2 {
            if let Ok(id) = parts[0].parse::<u32>() {
                let coords: Vec<&str> = parts[1].split(',').collect();
                let mut ov = CustomOverride::default();
                if coords.len() >= 1 { ov.x = coords[0].parse().ok(); }
                if coords.len() >= 2 { ov.y = coords[1].parse().ok(); }
                if coords.len() >= 3 { ov.w = coords[2].parse().ok(); }
                if coords.len() >= 4 { ov.h = coords[3].parse().ok(); }
                if coords.len() >= 5 { ov.scale = coords[4].parse().ok(); }
                map.insert(id, ov);
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_custom_layouts_empty() {
        let map = parse_custom_layouts("");
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_custom_layouts_valid() {
        let map = parse_custom_layouts("1:100,200,800,600,2;2:0,0,1920,1080,1");
        assert_eq!(map.len(), 2);
        let ov1 = map.get(&1).unwrap();
        assert_eq!(ov1.x, Some(100));
        assert_eq!(ov1.y, Some(200));
        assert_eq!(ov1.w, Some(800));
        assert_eq!(ov1.h, Some(600));
        assert_eq!(ov1.scale, Some(2));

        let ov2 = map.get(&2).unwrap();
        assert_eq!(ov2.x, Some(0));
        assert_eq!(ov2.y, Some(0));
        assert_eq!(ov2.w, Some(1920));
        assert_eq!(ov2.h, Some(1080));
        assert_eq!(ov2.scale, Some(1));
    }
}