/// deltaE calculation matching the CIE94 standard formula used in original Pick
pub fn delta_e(lab_a: &[f64; 3], lab_b: &[f64; 3]) -> f64 {
    let delta_l = lab_a[0] - lab_b[0];
    let delta_a = lab_a[1] - lab_b[1];
    let delta_b = lab_a[2] - lab_b[2];

    let c1 = (lab_a[1] * lab_a[1] + lab_a[2] * lab_a[2]).sqrt();
    let c2 = (lab_b[1] * lab_b[1] + lab_b[2] * lab_b[2]).sqrt();
    let delta_c = c1 - c2;

    let delta_h_sq = delta_a * delta_a + delta_b * delta_b - delta_c * delta_c;
    let delta_h = if delta_h_sq < 0.0 {
        0.0
    } else {
        delta_h_sq.sqrt()
    };

    let sc = 1.0 + 0.045 * c1;
    let sh = 1.0 + 0.015 * c1;

    let delta_l_kl_sl = delta_l / 1.0;
    let delta_c_kc_sc = delta_c / sc;
    let delta_h_kh_sh = delta_h / sh;

    let i = delta_l_kl_sl * delta_l_kl_sl
        + delta_c_kc_sc * delta_c_kc_sc
        + delta_h_kh_sh * delta_h_kh_sh;

    if i < 0.0 {
        0.0
    } else {
        i.sqrt()
    }
}
