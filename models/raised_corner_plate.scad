// Plate with a raised lower-left corner section.
// Includes three countersunk screw holes in the non-raised L-shaped region.

raised_width = 25;
raised_depth = 24;
raised_extra_height = 13;

base_width = 19 + raised_width;
base_depth = 19 + raised_depth;
base_height = 3;

screw_head_diameter = 5.5;
screw_shaft_diameter = 2.75;
screw_taper_height = 2.5;

// Rounding controls
shared_corner_radius = 2.5;  // shared base/raised lower-left corner side
inside_corner_radius = 2;    // inside corner of raised section (top-right of raised block)

hole_edge_offset = (base_width - raised_width) / 2;
hole_positions = [
  [hole_edge_offset, base_depth - hole_edge_offset],                  // top-left end
  [base_width - hole_edge_offset, base_depth - hole_edge_offset],     // top-right corner
  [base_width - hole_edge_offset, hole_edge_offset]                   // bottom-right end
];

module rounded_bl_rect(width, depth, radius) {
  union() {
    translate([radius, 0])
      square([width - radius, depth]);
    translate([0, radius])
      square([width, depth - radius]);
    translate([radius, radius])
      circle(r = radius, $fn = 64);
  }
}

module rounded_bl_tr_rect(width, depth, bl_radius, tr_radius) {
  union() {
    difference() {
      rounded_bl_rect(width, depth, bl_radius);
      translate([width - tr_radius, depth - tr_radius])
        square([tr_radius, tr_radius]);
    }

    translate([width - tr_radius, depth - tr_radius])
      intersection() {
        circle(r = tr_radius, $fn = 64);
        square([tr_radius, tr_radius]);
      }
  }
}

module countersunk_hole() {
  // Through shaft.
  translate([0, 0, -0.1])
    cylinder(h = base_height + 0.2, d = screw_shaft_diameter, $fn = 64);

  // Taper on the underside.
  taper_depth = min(screw_taper_height, base_height);
  translate([0, 0, -0.1])
    cylinder(
      h = taper_depth + 0.1,
      d1 = screw_head_diameter,
      d2 = screw_shaft_diameter,
      $fn = 64
    );
}

difference() {
  union() {
    // Bottom plate with rounded shared corner.
    linear_extrude(height = base_height)
      rounded_bl_rect(base_width, base_depth, shared_corner_radius);

    // Raised lower-left section with rounded shared corner and inside corner.
    linear_extrude(height = base_height + raised_extra_height)
      rounded_bl_tr_rect(raised_width, raised_depth, shared_corner_radius, inside_corner_radius);
  }

  // Three countersunk holes in the non-raised area.
  for (pos = hole_positions) {
    translate([pos[0], pos[1], 0])
      countersunk_hole();
  }
}
