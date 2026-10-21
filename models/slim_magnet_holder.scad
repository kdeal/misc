// Slim, enclosed holder for a 15mm diameter x 1.8mm button magnet.
// Units: mm. Print bottom-down; pause before the pocket roof to insert magnet.

$fn = 128;

actual_mag_dia = 15;
actual_mag_height = 1.8;
magnet_diameter_clearance = 0.5; // Total diametral clearance
magnet_height_clearance = 0.5;
magnet_dia = actual_mag_dia + magnet_diameter_clearance;
magnet_height = actual_mag_height + magnet_height_clearance;

bottom_thickness = 1;
cover_height = 5; // Fixed bottom-to-cap height; clearance uses roof thickness
above_magnet_thickness = cover_height - bottom_thickness - magnet_height;
assert(above_magnet_thickness > 0, "Magnet pocket must remain below the cap.");

// Preserve the original holder's outside diameters and cap height.
cover_diameter = 26.7;
cover_cap_overlap = 6; // Total added diameter (3mm per side)
cover_cap_height = 5;
edge_radius = 0.8; // Matching bottom and flat cap edge rounding

total_height = cover_height + cover_cap_height; // 10mm overall
body_radius = cover_diameter / 2;
cap_radius = (cover_diameter + cover_cap_overlap) / 2;

// A 45-degree underside replaces the unsupported horizontal cap overhang.
// Its 3mm rise is a conservative minimum for ordinary support-free FDM.
cap_taper_height = cap_radius - body_radius;
assert(cover_cap_height >= cap_taper_height + 2 * edge_radius,
       "Cap needs enough height for the taper and rounded rim.");

module rounded_outer_profile() {
  // Round the complete section before clipping to the rotation axis so
  // rounding cannot introduce a hole at the center of the top or bottom.
  intersection() {
    square([cap_radius, total_height]);
    offset(r=edge_radius)
      offset(delta=-edge_radius)
        polygon([
          [-body_radius, 0],
          [body_radius, 0],
          [body_radius, cover_height],
          [cap_radius, cover_height + cap_taper_height],
          [cap_radius, total_height],
          [-cap_radius, total_height],
          [-cap_radius, cover_height + cap_taper_height],
          [-body_radius, cover_height]
        ]);
  }
}

difference() {
  rotate_extrude()
    rounded_outer_profile();

  // Enclosed pocket, like the original: insert the magnet during printing.
  // With the defaults the pocket runs from Z=1mm to Z=3.3mm,
  // leaving 1.7mm of solid material below the cap at Z=5mm.
  translate([0, 0, bottom_thickness])
    cylinder(h=magnet_height, d=magnet_dia);
}
