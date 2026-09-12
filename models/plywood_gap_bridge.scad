// Plywood-gap bridge clip
// Units: mm
//
// The left end clamps one 1/2 in plywood panel. Its lower rail crosses the
// 1/8 in panel gap and supports the neighboring replacement panel from below.

part_length = 200;

plywood_thickness = 19.05;  // 1/2 in
board_gap = 3.175;         // 1/8 in
fit_clearance = 0.35;

wall_thickness = 3;
capture_depth = 20;
rest_depth = 20;
edge_taper = 8;

// The lower rail sits directly under both panels. The upper rail provides the
// clearance needed to slide the clamp over the fixed plywood panel.
lower_rail_top = 0;
lower_rail_bottom = lower_rail_top - wall_thickness;
upper_rail_bottom = plywood_thickness + fit_clearance;
upper_rail_top = upper_rail_bottom + wall_thickness;
replacement_end = -(board_gap + rest_depth);

module bridge_profile() {
  union() {
    // Lower rail under the captured panel, opening toward positive x.
    polygon(points = [
      [0, lower_rail_bottom],
      [capture_depth - edge_taper, lower_rail_bottom],
      [capture_depth, lower_rail_top],
      [0, lower_rail_top]
    ]);

    // The bridge projects from the wall in the opposite direction. It crosses
    // the panel gap and supports the replacement panel from below.
    polygon(points = [
      [replacement_end, lower_rail_top],
      [replacement_end + edge_taper, lower_rail_bottom],
      [0, lower_rail_bottom],
      [0, lower_rail_top]
    ]);

    // Outside wall of the capture slot.
    translate([0, lower_rail_top])
      square([wall_thickness, upper_rail_top - lower_rail_top]);

    // Upper rail: over the fixed panel only, completing the sliding clamp.
    // The tapered inner end makes the panel edge easier to insert.
    polygon(points = [
      [0, upper_rail_bottom],
      [capture_depth, upper_rail_bottom],
      [capture_depth - edge_taper, upper_rail_top],
      [0, upper_rail_top]
    ]);
  }
}

// Extruding the sideways-h profile makes a 200 mm-long bridge.
rotate([0, 90, 0])
  linear_extrude(height = part_length)
    bridge_profile();
