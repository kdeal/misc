// Drilling vacuum attachment adapter
// 10mm tall collar, OD 35mm, ID tapered from 30.75mm to 31mm.

$fn = 128;

outer_diameter = 35;
inner_diameter_bottom = 30.75;
inner_diameter_top = 31;
height = 10;
dot_count = 6;
dot_diameter = 2;
dot_height = 1;
dot_radius = (outer_diameter / 2) - 1.5;

difference() {
  union() {
    cylinder(h = height, d = outer_diameter);
    for (i = [0 : dot_count - 1]) {
      rotate([0, 0, i * 360 / dot_count])
        translate([dot_radius, 0, 0])
          cylinder(h = dot_height, d = dot_diameter);
    }
  }
  cylinder(h = height, d1 = inner_diameter_bottom, d2 = inner_diameter_top);
}
