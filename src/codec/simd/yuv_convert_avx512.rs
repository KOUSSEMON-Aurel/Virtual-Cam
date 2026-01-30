#![feature(portable_simd)]

use std::simd::*;

#[target_feature(enable = "avx512f")]
pub unsafe fn yuv420_to_yuyv_avx512(
    y: &[u8],
    u: &[u8],
    v: &[u8],
    yuyv: &mut [u8],
    width: usize,
    height: usize,
) {
    let width_simd = width & !63;
    
    for row in 0..height {
        let y_row = &y[row * width..];
        let uv_row_idx = row / 2;
        let u_row = &u[uv_row_idx * (width / 2)..];
        let v_row = &v[uv_row_idx * (width / 2)..];
        let yuyv_row = &mut yuyv[row * width * 2..];
        
        for x in (0..width_simd).step_by(64) {
            let y_vec = u8x64::from_slice(&y_row[x..x + 64]);
            
            // On charge 32 U et 32 V car ils sont sous-échantillonnés (4:2:0)
            let u_vec = u8x32::from_slice(&u_row[x/2..(x/2)+32]);
            let v_vec = u8x32::from_slice(&v_row[x/2..(x/2)+32]);
            
            // Entrelacement Y0 U0 Y1 V0 ...
            // Magie AVX-512 pour réorganiser les octets en un seul passage
            let result = interleave_avx512(y_vec, u_vec, v_vec);
            yuyv_row[x*2..(x+64)*2].copy_from_slice(&result);
        }
    }
}

#[inline(always)]
unsafe fn interleave_avx512(y: u8x64, u: u8x32, v: u8x32) -> [u8; 128] {
    let mut out = [0u8; 128];
    // Implementation réelle via intrinsics AVX-512...
    // Pour l'exemple, on simule une copie
    for i in 0..32 {
        out[i*4] = y[i*2];
        out[i*4+1] = u[i];
        out[i*4+2] = y[i*2+1];
        out[i*4+3] = v[i];
    }
    out
}
