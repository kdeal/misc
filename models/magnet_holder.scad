// Magnet Cover
// Units: mm

$fn = 128;

actual_mag_dia = 18.2;
actual_mag_height = 5;

magnet_dia = actual_mag_dia + 0.5;
magnet_height = actual_mag_height + 0.5;

// 28mm dia case, 3mm cap overhang, 9mm cap, 19.5mm tall
wall_thickness = 4;
bottom_thickness = 2;

cover_diameter = magnet_dia + (2 * wall_thickness);
cover_height = 10;
cover_cap_height = 9;
cover_cap_overlap = 3;

difference() {
  union() {
   cylinder(h=cover_height, d=cover_diameter);
   translate([0,0, cover_height])
    intersection() {
      cylinder(h=cover_cap_height, d=cover_diameter + cover_cap_overlap);
      // Top round
      translate([0, 0, -11.75])
      sphere(d=cover_diameter + cover_cap_overlap + 10);
    } 
  }
  translate([0,0, bottom_thickness])
  cylinder(h=magnet_height, d=magnet_dia);
}
