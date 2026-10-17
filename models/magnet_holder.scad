// Magnet Cover
// Units: mm

$fn = 128;

actual_mag_dia = 18.2;
actual_mag_height = 5;

magnet_dia = actual_mag_dia + 0.5;
magnet_height = actual_mag_height + 0.5;

// 26.7mm dia case, 3mm cap overhang per side, 5mm cap, 15mm tall
wall_thickness = 4;
bottom_thickness = 1;

cover_diameter = magnet_dia + (2 * wall_thickness);
cover_height = 10;
cover_cap_height = 5;
cover_cap_overlap = 6; // Total added diameter (3mm per side)
cover_cap_edge_radius = 0.8;

module dome_cap() {
  cap_radius = (cover_diameter + cover_cap_overlap) / 2;

  // Round the rim of a half-ellipse, then revolve it into a shallow dome.
  // Round across the axis before clipping so the dome stays solid at its center.
  rotate_extrude()
    intersection() {
      square([cap_radius, cover_cap_height]);
      offset(r=cover_cap_edge_radius)
        offset(delta=-cover_cap_edge_radius)
          intersection() {
            scale([cap_radius, cover_cap_height]) circle(r=1);
            translate([-cap_radius, 0])
              square([2 * cap_radius, cover_cap_height]);
          }
    }
}

difference() {
  union() {
   cylinder(h=cover_height, d=cover_diameter);
   translate([0,0, cover_height])
     dome_cap();
  }
  translate([0,0, bottom_thickness])
  cylinder(h=magnet_height, d=magnet_dia);
}
