mod clip_path;
mod container;
mod defs;
mod dom;
pub mod fe;
mod filter;
mod g;
mod gradient;
mod image;
mod mask;
mod node;
mod node_hierarchy;
mod pattern;
mod shape;
mod stop;
mod svg_;
mod text;
mod transformable_node;
mod types;
mod r#use;

pub use self::{
    clip_path::ClipPath,
    container::Container,
    defs::Defs,
    dom::*,
    filter::Filter,
    g::G,
    gradient::*,
    image::Image,
    mask::Mask,
    node::*,
    node_hierarchy::*,
    r#use::Use,
    shape::*,
    stop::Stop,
    svg_::*,
    text::{TSpan, Text, TextLiteral, TextPath},
    transformable_node::TransformableNode,
    types::*,
};

#[macro_export]
macro_rules! impl_default_make {
    ($type:ty, $make_fn:path) => {
        impl Default for $type {
            fn default() -> Self {
                Self::from_ptr(unsafe { $make_fn() }).unwrap()
            }
        }
    };
}

#[cfg(test)]
mod tests {

    use super::Dom;
    use crate::{
        prelude::{NativeAccess, NativeRefCounted},
        resources::NativeResourceProvider,
        surfaces,
        svg::{Length, LengthUnit},
        FontMgr, Surface,
    };

    #[test]
    fn render_simple_svg() {
        // https://dev.w3.org/SVG/tools/svgweb/samples/svg-files/410.svg
        // Note: height and width must be set
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" height = "256" width = "256">
            <path d="M30,1h40l29,29v40l-29,29h-40l-29-29v-40z" stroke="#;000" fill="none"/>
            <path d="M31,3h38l28,28v38l-28,28h-38l-28-28v-38z" fill="#a23"/>
            <text x="50" y="68" font-size="48" fill="#FFF" text-anchor="middle"><![CDATA[410]]></text>
            </svg>"##;
        let mut surface = surfaces::raster_n32_premul((256, 256)).unwrap();
        let canvas = surface.canvas();
        let dom = Dom::from_str(svg, FontMgr::new()).unwrap();
        dom.render(canvas);
        save_to_tmp(&mut surface, "simple");
    }

    #[test]
    fn resource_provider_and_font_mgr_get_dropped_after_drop_of_dom() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" height = "256" width = "256">
            </svg>"##;
        let mut surface = surfaces::raster_n32_premul((256, 256)).unwrap();
        let canvas = surface.canvas();
        let font_mgr = FontMgr::new();
        let provider: NativeResourceProvider = font_mgr.clone().into();
        let dom = Dom::from_str(svg, provider.clone()).unwrap();
        dom.render(canvas);
        // Dom keeps the resource provider even after rendering.
        assert!(provider.native()._ref_cnt() >= 2);
        // And at least two of the font managers are referred to (one in the resource provider, and the other in the Dom)
        assert!(font_mgr.native()._ref_cnt() >= 3);
        drop(dom);
        // now it's free.
        assert_eq!(1, provider.native()._ref_cnt());
        drop(provider);
        // and so is the font mgr
        assert_eq!(1, font_mgr.native()._ref_cnt());
    }

    // Run this manually (needs network connectivity)
    #[cfg(feature = "ureq")]
    #[test]
    #[ignore]
    fn render_svg_with_ureq_resource_provider() {
        use crate::resources::UReqResourceProvider;

        let svg = r##"
            <svg version="1.1"
            xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
            width="128" height="128">
            <image width="128" height="128" transform="rotate(45)" transform-origin="64 64"
                xlink:href="https://www.rust-lang.org/logos/rust-logo-128x128.png"/>
            </svg>"##;
        let mut surface = surfaces::raster_n32_premul((256, 256)).unwrap();
        let canvas = surface.canvas();
        let resource_provider = UReqResourceProvider::new(FontMgr::new());
        let dom = Dom::from_str(svg, resource_provider).unwrap();
        dom.render(canvas);
        save_to_tmp(&mut surface, "svg-with-ureq");
    }

    // A test case to see if a download error is handled.
    #[cfg(feature = "ureq")]
    #[test]
    fn render_svg_with_ureq_resource_provider_with_missing_resource() {
        use crate::resources::UReqResourceProvider;

        let svg = r##"
            <svg version="1.1"
            xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
            width="128" height="128">
            <image width="128" height="128" transform="rotate(45)" transform-origin="64 64"
                xlink:href="https://www.not-existing.org/logos/rust-logo-128x128.png"/>
            </svg>"##;
        let mut surface = surfaces::raster_n32_premul((256, 256)).unwrap();
        let canvas = surface.canvas();
        let resource_provider = UReqResourceProvider::new(FontMgr::new());
        let dom = Dom::from_str(svg, resource_provider).unwrap();
        dom.render(canvas);
        save_to_tmp(&mut surface, "svg-with-ureq-missing-image");
    }

    // data: (png image taken from <https://stackoverflow.com/questions/5242319/what-does-this-mean-image-pngbase64>)
    #[test]
    fn svg_with_base64_image_with_escaped_encoding() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="256" height="256">
            <image width="256" height="256" xlink:href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAKsAAADVCAMAAAAfHvCaAAAAGFBMVEVYn%2BH%2F%2F%2F%2Bex%2B3U5vd7s%2Bfq8%2Fs0itq72PMLUPvtAAASvklEQVR4AbXBC0JqCQxEwT5Jd7L%2FHc8FdR4g%2BEGtEr8u%2FBHxu7otdzd%2FQPyqlmRp1Pw%2B8aukDfRa1fw28ZtWy4sa89vEb7LCi0zx28RvqgkvouW3id%2FU8pbtWmv5beJXRWNrRmp%2BnfhlHXZm%2BQPi95Vk%2FoD4fZbMHxC%2FryTzB8Tva435A%2BL3rcb8AfH7VjJ%2FQPy%2BHYk%2FIH5facwfEL8iaZcrnKyn%2BAPi57K2VL2WF1hJ%2FAHxQ2tJrg6HteXVjPkD4ge6V3J1%2BF97zhx%2BnXhWb8nacKXlnYPErxNPyfqw4ZYKVuUZdfhd4hmxunY73NICgfWMOvwm8ZQ1pMvlDZdaCic98kjV4beIp8ScpLvsSvhflzqQmqVLB281v0E8pc2bdNne8EayNTPNSbt02PBj4intcKltb%2FNibY%2BLf9aSO%2FyMeMo6XMva3g0vwrWsxvyMeEoc3knZ2g53ZaXa8DzxlHa4J23Jae5aycXTxFPa4WRdXAtdsivckZXG4TniKWtOSlre6y7LG651Wxq5OzxDPGUVIKNwX6ekCv%2B0ddglVPMM8ZQ10FJ4LGVvOEuXRl7OqnmGeEor4Ck%2BtnI1ZEvjDa%2FcPEM8ZQVY4RO9VqUlN%2F84PEM8JQ50cUgXH2mrKlyq5RniOQ4vVjPLHdu86OKGi2eIr%2BgNV6JwljmYO6zlbJsbWp4hPtVrjYpLLV7UHIp7rOVkixtaniE%2BU5I2Nc2FKJytZhTuiac5rLnh4hniEzUbDjXhn3g5W0nNA1aAKm7YPEN8bMecrZYLWl70hkcyBay5YfMM8aHI4aR7xAUVHyirOdhAmRsqniE%2BtOKsRjIXtDzmmRGHVmDFDRfPEB%2BJzMmO01xScdYnVRs6vPHMFG9W4ZrMM8RHouWw43DNhlDWiSVZY3nDoWYc3qzDNZlniPe6w4uoOFjcKhPXuJNWyG6VqjSuhm7%2BiZorUfEM8U5J8nKyMw0tcZLwPxdRtTlUcUgVdGlml0uZ4pqKZ4hr5VUnpSXdUgVa4hA5vHERV1Tp9XhdJTWHksYd%2Ftdarql4hrjQiaPiYLclNSeebVYz5o0W7Ghsa9blmlFtx01rxP8yy5XIPEP8L1W7bjWHlbzhRTwjzXrCK1f3qqSEyBysLVtayKp40yqurcITxJtUgavVHNob%2FinZTWt5VVvWVKvJSttQCkRjb%2FA4vLK5thOeIN6sm9ai5cTFhYRDy%2FyTGpdU0hxkaZvWUrZluTmLims14QniVbywClqgeouT9IZXNWoupGzNqHa3y5LGVYBnipbCSVxcq1meIN54oRXsbEk26S3NmBcZ807K3gon2ZLcxF5tPMVJprlWE54g3nihtbRHm7WjkbxTHSCWwj1r2U4HSMmdQEmWwonNtah4gnhjA9ZSaohmpnpDjWRptDwS25LcQGsc2Bla5sTFtZV4gnixpWmIVWpgRuVwsiV5q7kv0JJcNVIFapydUrHTQKa5IfMEcRKrurSQ0qhsmVR4kea%2B7pIr9NqSrRltWlaxomUgVVyLxBPEYeUGygtszew2KfOBclVpVN2ctCXNidZaaKWmONhc6rKaJwi6xuGkRmWpAkRa7outF9XN%2F7LlmbJmpiCyvBxk%2FtnSqHmGWGk5i2ZcaWBLau5KKHt3Ce%2FsaLMz46VG4cTFm%2FaMOzxFUYWztjzhkNI43JPyYvPAegPxzFRpOYmWF1WywrPUag5xjRapqqxxubijvYFVaC%2Fv7YSDpzxjzlbhpKXxhqcpWshqtECk0Yys6m5utZdD1LCuCifhfyVOapqsxhyiQMmSm58QNdZheZGV5FqwueXiZBUga28DvRte1NQCpQVSUkFqPbIr%2FIxg7arwJqqEg6e5Vuas1Zytyw1ka5uT9ajKI87WbksaLT8mbkXFyWqaa2rOVuFVStUNpGrDoSTPmDfWdlby8kPiHQtoa0vLpXU4WzX%2FS5W2gWxtOHQ24U3CSUmu8BPinR2XVSFyuNAOZ9Fyae1qDu2qcF8suRKeJt7pcW1zaE9xwcVZq7nWtpeTrQ0PrEeq8CTxnsWrlbThELra5ixqbsXWNoeq6nBft6TlOeK9VnG2lfb4TKOOlpOouKPsWg4pb3Nf1uMGusP3iDtKDaTcgMuWvL1FmZOouCtlbwJs1Yb7SuN2Nd8k7mgvXV4OKWALiGkVJ14eyPqQQG9Vc0dWGnn5LnFPTW1z1gW0OdSyag5aHsvaroVs1YZL2dKMt1nzXeKulas52QLanGy3xq4a87Eu2yHZ2uZNWzPjDbDmu8R9a8m7iQNscbKyy%2BWS%2BUzWtqp7qzpA1jPj8KKK7xIPZG2NVWTTSbpKbs5cfEF6y64qV6ctqcKbdvgm8VhSlnWwJbuaV3LzRb11onFt%2BKcVvkl8one7u3bD%2FzJuXnRt%2BFTXVHOWqubQ4rvEEyI1L1Z2h8%2B0eRHLKiBqvkk8IePmxZq1lk%2B0w0nJUHKIlm8ST8ioeVEFtFwbPhA3h8gcdpZV803iCRkVL7Y42bK2w0NlDqXlpJRV803iGZYrnFRxlqwO3eEuN4dSOGlVme8Sz7C37QZqeZPekl0b3nMBreKsp1bNN4lnWIEtF1Vc6i1bVZtwxQX0NC9UrfBN4hk7zaHLNrey1kgVLljATnO2rmj5JvEMqzlrF%2B%2BFXitcsAArnFkdLd8knrFqPmFzyQq0xUm0tJZvEs8oAR0eix0u1ARSqg70NNHyTeIZUqgZ85gdLlgcMjOSRlBqvkk8wwOSp3moJlyoCYfeKkmBVvgm8YyaUJJ5zOJSTXMWSgus%2BC7xjJpA%2BMiquVATXiUcSuGbxDNqmk%2BUxtW82WmurMI3iWd4wifaHo1rNxx2miul8E3iGTXhc4nH0lQ1O80VK3yTeEYNX5SspbEnXFmFbxLPqGm%2BrsvWFFdK4ZvEM2rCt6RmzCWL7xLP2Anfs2M3Fyy%2BSzyjpvmqDoed5YrFd4ln7DRftHI19BRXSuGbxDN6wtdEqjF4lisS3yWeEYUvWlkDNeZKTfgm8ZFu7mqFr%2FKMYae4lFH4JvGBVLgraghf09uQMZdabr5JfKC2q1zV3IgarOLLPMWllptvEo%2B1e7dkq5ZrLkip%2BKqa4lLk5ZvEY15INay9XIqXVGS%2BqsdcirzclYVa7hAPbQFVnJSaC9HCapavqjGXIjXvbNmSxi7eE4%2BsA21OumwuSQUJX1ZjLsVabqR6t7tUlrThhnjEC%2FFy6AKbCy45zdftmEutKm5UcSgHspY7XBEPVAFVHLoCUXPFkr3hi2wutba44QDr5iyeqQ3%2FiAccqOLQDhAV17pG0jZfUuZS5OJaGYiWF%2B2ypOV%2F4q5UQZtDu4G2xK10aeTlC1bhUslciQpYh7PSQtau8ErcVYZ4gXYDcUXLe1lrvBU%2B0VoutFRcWQWo4qwdTlYSr8Q9caDMwc3BDgl3xZpRb%2FORnuVCJHNlla2oOYmLQ8q7Ll6Ie6pgDaQKSCl8IF3WqAgPrbgU2VxpV1kje2EdoOWGlsOJuKMd1g14OdjNp1YjNY%2B0m0s15kYgJVlaFxBVOETuAOK9eEELrDmUli%2Fo8oy94S4Xl2LzQGukEFU46RptQLy3BWWgHSBTvEp32eGRtjTjSriQBKLlShUPrSRcnK2qtIB4Zw3tQNRAbF5FB0vhoS57JFXzZmUtuLiy5gNlTTixlkgB8Y4byhAX0HJ4Y%2FcmWkjz0NrSaMNJ5EiNi3%2FSpPlIayqA3UBcIG5tQTuwBcQOJx3AsrSzxHJ4bKs9U5xoqWnK4U17%2BUzPFLQ4iQ3iRtxQC3gBK5xZJjOutcaSpeYjsUZqKFmGOLxIaflU1jI2ZzuLuLGuLe2yBlrLC1tdWg7ZmWal8KHeGtXG0gLLSdZyha%2BoKYdDl7WIGxpbI7lSicyLqFkH2rVZF%2BwUnymNXNu8WUkVLqSaB6IpIGWXF3Ft1UC6rRq3mhc7TRXgLS2lrKb5VEoz6nCSrtE2V6p4aMeQ8tJaxLU4nGU9o%2BXVTrMF%2BLBgjYqvSNkjL%2BDxhmut5tDb3CF1uwJoEdday6vMTHjVs7GA3g3QU8tXxZJc6Q23yhxWckPCtZW1nLgQ12KFF5Ed3pQ0U7yKp%2Fi6YM%2FI4dZOA3FRRdvhSmaWMxtxI3JzVlP8k9qsVFWdbVvTfENCjcytUoBW46XscE3DizLi1o6KQ4%2FDlZRsWSfBCt%2BSdHGrzGHFOtjFtUgNNJQR78Qjr%2BVwzV4I65SazPJzrQbKq6bl5kapU7bbRryXLo3c3LATYIfMEs3yc1bA44bScqumvJ21jLgrhHdSktNWkONR%2BLmULMnbpQm3pOWkZxHf0R7NKKykDr9iq3ptuexOuJQRZ5lCfE96K5Ct5iNpe118WQKxVeGCxnYDmUL8iUjb2%2BXmexIu9Di9XtgpxJ9wcehuOzwt1gJx4ynEM9K9tS5X7fLempP2dmnDczwjTlLYi%2FiCnHXSe9LWic9k3qvlRTltLU%2Bp2lE1sKUG8bm2DiNpNBpJu5vwwuEdLa%2FWy6p4JL27Dg%2B0pUBsQHxu67C1Vb2dpLlU5h3bG87aS0vNXWtJtip0bbjDhqgB8TkvH1g115qttnfDoW0oNe%2B1Rs0hlqVRc8cSmYP4XBUfUXHNlQ5tqzkpNaXmHVV4lVpq1NxjhYP43JqP2FwracOh7OZQDuXmRmu5sjMO75SWE%2FE5F4%2F09s5wI5abQ0rFoVxZNTes7e7wvy053NpwJj7n4kVCDt29teWypJHFOy0VJ6sN0CrK4dpakmv5pxQeEZ8rQ%2B9alnU2knyo2k64Ix4vh5I5sVNarqW3u8z%2F4mkeEZ8LrCxXtbfWu9t8qqQK0DKHVtEubrWm%2BZ9VPCS%2BJN1828oB4gqwalrFtUjNP3bzkPg7sdXAyhyssF4upWb5Z8c8Jv5QWmpgVRxUsGoulMw%2FPQqPiZ%2Fp8JGVOLQWKAW6%2BCcyF2qGD4gfibe2ead5lXEDpQAu0rv8r2WgtZxl1Twm%2Ftls1HxHK7HDjZV51VIgWmBlSeMKr%2BxseZYXq%2BUx8aY0MxrvVnUC4XNxgYtrJY15taMmNlAztd0lhxfW6MChC1rFY%2BLVjlwzKutVdfhE7xjKXEiX3CuHFzWG0lLycogUXnTtxuaws6DiMfFK09kZQ9K1VSvJ3oRHslIFWuGftdzQUoWzlYONinBILRdaC8TTYPO%2F3nBFnKxLG2um%2BKfXOrg6vBdLrvJSCm9SJpy0RtucrMRq1Zy1woUy0B4HbN60ex0uiEN0KLk1xZXs2paKW9FIqrJrzP%2Fs5k17tJz0GE%2FxohwulGElOUTmTRWl5oI4lKRRsTPhVpIdc6sl10IsFW9WXNpROPH0TkGAVnFpx5a63WSKN5HVXBKwc1btEffsNO8kvBObS5lZTnaUMXFYqbnUltwg75h%2FusMVATXleWW7qk1Xb8KLVfiKlsIlj9Sc1FhFtjITboSTlSp8QMCO5JU11bb1ZlQdIHL4iprmktWROclqGlaWmvsSPiRAIy3lcAhk05vsWgfbU3xFVFyRWTUvSqqa2S7zHEFmRikt7yS18kxxFj6yY67UbNu86U6qIApPEUSasZb7Ek0DqXh5LHa4lDFWc6kd4uUpgsiaKR6pKQ61uHmsZrmyk1ZxpQ1oeYoAzaG4ry1zsuXisVJxpeQdc60N2DxFgGckc1ePixdpHkjVjrnS0kpc6u5SwMtTBKxkybUJN3bUfCaulsMVTVvNP%2BmyNQVe7tjlE%2BJFb1mSLVfV9jaHHS2fiao15sqOd4pL29ArbxXvldV8TPwv6XVV6YXtGTefiiqaMRei2TFXKpzUONxKFWo%2BJt5J0ltlzQxfsCqimSpv86KmrHApBbXA2s2NuKPwMfFQvOELWgvsnEjVQMYtc2UXqjm0xI0yq%2FAx8T0JtyJz8DiekWpjxWoupRqqOamp5VJPsXJt9256wz3iW8oOt1xNaWah3NZJZK7UAg6HLo%2B5tFPgke2SreUe8R1rO9xayTpALFaa2Z3mUhyo4qQ6I67MbLlsyyfFPeI71m7ey0orw2pL256WuFILVHOI41mu1IyK3u0q28094nvCXQHLtqyF9Gq5tA7E4bAViRsrNW%2FCXeK3lDTVVoBI4ZIDVHFYpbTcyIbPiF%2FTSbPT3SUtl6qAuDl4W8UzxC%2Fz6CRciALUcijT4inil%2FV2p4pLtUDcwCol8xTxF8KlKg5VQGtb4jniz7UbWAcox%2BJJ4s%2B5OLiAVnuKJ4m%2FtuawBURbszxL%2FLF4OXgh9s7yNPHHqjisgVLLPE%2F8rXYD7UCrVsXzxN%2Bq4uAGrFj8gPhTXRzKwGprmh8Qf2rlot2AvSp%2BQvyl1nikAlprh58Qf0lqolGBarX8iPhLZWBVqnVsfkb8pTaHcru61PyM%2BEtrDq2UW8sPib%2FUChBvbIcfEn%2FKxWGrpeWnxJ9qVYDyVPgp8bfa2qRmmh8Tf21lq5qfE38uveE3%2FAdr385%2FSVd%2FMAAAAABJRU5ErkJggg%3D%3D"/>
            </svg>"##;
        let mut surface = surfaces::raster_n32_premul((256, 256)).unwrap();
        let canvas = surface.canvas();
        let dom = Dom::from_str(svg, FontMgr::new()).unwrap();
        dom.render(canvas);
        save_to_tmp(&mut surface, "svg-with-base64-image-escaped-encoding");
    }

    // data: (gif image from <https://www.rfc-editor.org/rfc/rfc2397>)
    #[test]
    fn svg_with_base64_image2() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="256" height="256">
            <image width="256" height="256" xlink:href="data:image/gif;base64,R0lGODdhMAAwAPAAAAAAAP///ywAAAAAMAAw
                AAAC8IyPqcvt3wCcDkiLc7C0qwyGHhSWpjQu5yqmCYsapyuvUUlvONmOZtfzgFz
                ByTB10QgxOR0TqBQejhRNzOfkVJ+5YiUqrXF5Y5lKh/DeuNcP5yLWGsEbtLiOSp
                a/TPg7JpJHxyendzWTBfX0cxOnKPjgBzi4diinWGdkF8kjdfnycQZXZeYGejmJl
                ZeGl9i2icVqaNVailT6F5iJ90m6mvuTS4OK05M0vDk0Q4XUtwvKOzrcd3iq9uis
                F81M1OIcR7lEewwcLp7tuNNkM3uNna3F2JQFo97Vriy/Xl4/f1cf5VWzXyym7PH
                hhx4dbgYKAAA7"/>
            </svg>"##;
        let mut surface = surfaces::raster_n32_premul((256, 256)).unwrap();
        let canvas = surface.canvas();
        let dom = Dom::from_str(svg, FontMgr::new()).unwrap();
        dom.render(canvas);
        save_to_tmp(&mut surface, "svg-with-base64-image2");
    }

    #[cfg(feature = "save-svg-images")]
    fn save_to_tmp(surface: &mut Surface, name: &str) {
        use crate::EncodedImageFormat;
        use std::{fs::File, io::Write, path::Path};

        let image = surface.image_snapshot();
        let data = image.encode(None, EncodedImageFormat::PNG, None).unwrap();
        write_file(data.as_bytes(), Path::new(&format!("/tmp/svg-{name}.png")));

        pub fn write_file(bytes: &[u8], path: &Path) {
            let mut file = File::create(path).expect("failed to create file");
            file.write_all(bytes).expect("failed to write to file");
        }
    }

    #[cfg(not(feature = "save-svg-images"))]
    fn save_to_tmp(_surface: &mut Surface, _name: &str) {}

    #[test]
    fn test_svg_attributes() {
        let data = r#"
            <svg width="100" height="200" viewBox="0 0 1200 800" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xml:space="preserve" xmlns:serif="http://www.serif.com/" style="fill-rule:evenodd;clip-rule:evenodd;stroke-linejoin:round;stroke-miterlimit:1.41421;">
                <g id="Layer-1" serif:id="Layer 1">
                    <g transform="matrix(1,0,0,1,597.344,637.02)">
                        <path d="M0,-279.559C-121.238,-279.559 -231.39,-264.983 -312.939,-241.23L-312.939,-38.329C-231.39,-14.575 -121.238,0 0,0C138.76,0 262.987,-19.092 346.431,-49.186L346.431,-230.37C262.987,-260.465 138.76,-279.559 0,-279.559" style="fill:rgb(165,43,0);fill-rule:nonzero;"/>
                    </g>
                    <g transform="matrix(1,0,0,1,1068.75,575.642)">
                        <path d="M0,-53.32L-14.211,-82.761C-14.138,-83.879 -14.08,-84.998 -14.08,-86.121C-14.08,-119.496 -48.786,-150.256 -107.177,-174.883L-107.177,2.643C-79.932,-8.849 -57.829,-21.674 -42.021,-35.482C-46.673,-16.775 -62.585,21.071 -75.271,47.686C-96.121,85.752 -103.671,118.889 -102.703,120.53C-102.086,121.563 -94.973,110.59 -84.484,92.809C-60.074,58.028 -13.82,-8.373 -4.575,-25.287C5.897,-44.461 0,-53.32 0,-53.32" style="fill:rgb(165,43,0);fill-rule:nonzero;"/>
                    </g>
                    <g transform="matrix(1,0,0,1,149.064,591.421)">
                        <path d="M0,-99.954C0,-93.526 1.293,-87.194 3.788,-80.985L-4.723,-65.835C-4.723,-65.835 -11.541,-56.989 0.465,-38.327C11.055,-21.872 64.1,42.54 92.097,76.271C104.123,93.564 112.276,104.216 112.99,103.187C114.114,101.554 105.514,69.087 81.631,32.046C70.487,12.151 57.177,-14.206 49.189,-33.675C71.492,-19.559 100.672,-6.755 135.341,4.265L135.341,-204.17C51.797,-177.622 0,-140.737 0,-99.954" style="fill:rgb(165,43,0);fill-rule:nonzero;"/>
                    </g>
                    <g transform="matrix(-65.8097,-752.207,-752.207,65.8097,621.707,796.312)">
                        <path d="M0.991,-0.034L0.933,0.008C0.933,0.014 0.933,0.02 0.933,0.026L0.99,0.069C0.996,0.073 0.999,0.08 0.998,0.087C0.997,0.094 0.992,0.1 0.986,0.103L0.92,0.133C0.919,0.139 0.918,0.145 0.916,0.15L0.964,0.203C0.968,0.208 0.97,0.216 0.968,0.222C0.965,0.229 0.96,0.234 0.953,0.236L0.882,0.254C0.88,0.259 0.877,0.264 0.875,0.27L0.91,0.33C0.914,0.336 0.914,0.344 0.91,0.35C0.907,0.356 0.9,0.36 0.893,0.361L0.82,0.365C0.817,0.369 0.813,0.374 0.81,0.379L0.832,0.445C0.835,0.452 0.833,0.459 0.828,0.465C0.824,0.47 0.816,0.473 0.809,0.472L0.737,0.462C0.733,0.466 0.729,0.47 0.724,0.474L0.733,0.544C0.734,0.551 0.731,0.558 0.725,0.562C0.719,0.566 0.711,0.568 0.704,0.565L0.636,0.542C0.631,0.546 0.626,0.549 0.621,0.552L0.615,0.621C0.615,0.629 0.61,0.635 0.604,0.638C0.597,0.641 0.589,0.641 0.583,0.638L0.521,0.602C0.52,0.603 0.519,0.603 0.518,0.603L0.406,0.729C0.406,0.729 0.394,0.747 0.359,0.725C0.329,0.705 0.206,0.599 0.141,0.543C0.109,0.52 0.089,0.504 0.09,0.502C0.093,0.499 0.149,0.509 0.217,0.554C0.278,0.588 0.371,0.631 0.38,0.619C0.38,0.619 0.396,0.604 0.406,0.575C0.406,0.575 0.406,0.575 0.406,0.575C0.407,0.576 0.407,0.576 0.406,0.575C0.406,0.575 0.091,0.024 0.305,-0.531C0.311,-0.593 0.275,-0.627 0.275,-0.627C0.266,-0.639 0.178,-0.598 0.12,-0.566C0.055,-0.523 0.002,-0.513 0,-0.516C-0.001,-0.518 0.018,-0.533 0.049,-0.555C0.11,-0.608 0.227,-0.707 0.256,-0.726C0.289,-0.748 0.301,-0.73 0.301,-0.73L0.402,-0.615C0.406,-0.614 0.41,-0.613 0.415,-0.613L0.47,-0.658C0.475,-0.663 0.483,-0.664 0.49,-0.662C0.497,-0.66 0.502,-0.655 0.504,-0.648L0.522,-0.58C0.527,-0.578 0.533,-0.576 0.538,-0.574L0.602,-0.608C0.608,-0.612 0.616,-0.612 0.623,-0.608C0.629,-0.605 0.633,-0.599 0.633,-0.592L0.637,-0.522C0.642,-0.519 0.647,-0.515 0.652,-0.512L0.721,-0.534C0.728,-0.536 0.736,-0.535 0.741,-0.531C0.747,-0.526 0.75,-0.519 0.749,-0.512L0.738,-0.443C0.742,-0.439 0.746,-0.435 0.751,-0.431L0.823,-0.439C0.83,-0.44 0.837,-0.437 0.842,-0.432C0.847,-0.426 0.848,-0.419 0.845,-0.412L0.821,-0.347C0.824,-0.342 0.828,-0.337 0.831,-0.332L0.903,-0.327C0.911,-0.327 0.917,-0.322 0.92,-0.316C0.924,-0.31 0.924,-0.302 0.92,-0.296L0.883,-0.236C0.885,-0.231 0.887,-0.226 0.889,-0.22L0.959,-0.202C0.966,-0.2 0.972,-0.195 0.974,-0.188C0.976,-0.181 0.974,-0.174 0.969,-0.168L0.92,-0.116C0.921,-0.111 0.923,-0.105 0.924,-0.099L0.988,-0.068C0.995,-0.065 0.999,-0.059 1,-0.052C1.001,-0.045 0.997,-0.038 0.991,-0.034ZM0.406,0.575C0.406,0.575 0.406,0.575 0.406,0.575C0.406,0.575 0.406,0.575 0.406,0.575Z" style="fill:url(#_Linear1);fill-rule:nonzero;"/>
                    </g>
                    <g transform="matrix(1,0,0,1,450.328,483.629)">
                        <path d="M0,167.33C-1.664,165.91 -2.536,165.068 -2.536,165.068L140.006,153.391C23.733,0 -69.418,122.193 -79.333,135.855L-79.333,167.33L0,167.33Z" style="fill-rule:nonzero;"/>
                    </g>
                    <g transform="matrix(1,0,0,1,747.12,477.333)">
                        <path d="M0,171.974C1.663,170.554 2.536,169.71 2.536,169.71L-134.448,159.687C-18.12,0 69.421,126.835 79.335,140.497L79.335,171.974L0,171.974Z" style="fill-rule:nonzero;"/>
                    </g>
                    <g transform="matrix(-1.53e-05,-267.211,-267.211,1.53e-05,809.465,764.23)">
                        <path d="M1,-0.586C1,-0.586 0.768,-0.528 0.524,-0.165L0.5,-0.064C0.5,-0.064 1.1,0.265 0.424,0.731C0.424,0.731 0.508,0.586 0.405,0.197C0.405,0.197 0.131,0.376 0.14,0.736C0.14,0.736 -0.275,0.391 0.324,-0.135C0.324,-0.135 0.539,-0.691 1,-0.736L1,-0.586Z" style="fill:url(#_Linear2);fill-rule:nonzero;"/>
                    </g>
                    <g transform="matrix(1,0,0,1,677.392,509.61)">
                        <path d="M0,-92.063C0,-92.063 43.486,-139.678 86.974,-92.063C86.974,-92.063 121.144,-28.571 86.974,3.171C86.974,3.171 31.062,47.615 0,3.171C0,3.171 -37.275,-31.75 0,-92.063" style="fill-rule:nonzero;"/>
                    </g>
                    <g transform="matrix(1,0,0,1,727.738,435.209)">
                        <path d="M0,0.002C0,18.543 -10.93,33.574 -24.408,33.574C-37.885,33.574 -48.814,18.543 -48.814,0.002C-48.814,-18.539 -37.885,-33.572 -24.408,-33.572C-10.93,-33.572 0,-18.539 0,0.002" style="fill:white;fill-rule:nonzero;"/>
                    </g>
                    <g transform="matrix(1,0,0,1,483.3,502.984)">
                        <path d="M0,-98.439C0,-98.439 74.596,-131.467 94.956,-57.748C94.956,-57.748 116.283,28.178 33.697,33.028C33.697,33.028 -71.613,12.745 0,-98.439" style="fill-rule:nonzero;"/>
                    </g>
                    <g transform="matrix(1,0,0,1,520.766,436.428)">
                        <path d="M0,0C0,19.119 -11.27,34.627 -25.173,34.627C-39.071,34.627 -50.344,19.119 -50.344,0C-50.344,-19.124 -39.071,-34.627 -25.173,-34.627C-11.27,-34.627 0,-19.124 0,0" style="fill:white;fill-rule:nonzero;"/>
                    </g>
                    <g transform="matrix(-1.53e-05,-239.021,-239.021,1.53e-05,402.161,775.388)">
                        <path d="M0.367,0.129C-0.364,-0.441 0.223,-0.711 0.223,-0.711C0.259,-0.391 0.472,-0.164 0.472,-0.164C0.521,-0.548 0.525,-0.77 0.525,-0.77C1.203,-0.256 0.589,0.161 0.589,0.161C0.627,0.265 0.772,0.372 0.906,0.451L1,0.77C0.376,0.403 0.367,0.129 0.367,0.129Z" style="fill:url(#_Linear3);fill-rule:nonzero;"/>
                    </g>
                </g>
                <defs>
                    <linearGradient id="_Linear1" x1="0" y1="0" x2="1" y2="0" gradientUnits="userSpaceOnUse" gradientTransform="matrix(1,0,1.38778e-17,-1,0,-0.000650515)"><stop offset="0" style="stop-color:rgb(247,76,0);stop-opacity:1"/><stop offset="0.33" style="stop-color:rgb(247,76,0);stop-opacity:1"/><stop offset="1" style="stop-color:rgb(244,150,0);stop-opacity:1"/></linearGradient>
                    <linearGradient id="_Linear2" x1="0" y1="0" x2="1" y2="0" gradientUnits="userSpaceOnUse" gradientTransform="matrix(1,0,0,-1,0,1.23438e-06)"><stop offset="0" style="stop-color:rgb(204,58,0);stop-opacity:1"/><stop offset="0.15" style="stop-color:rgb(204,58,0);stop-opacity:1"/><stop offset="0.74" style="stop-color:rgb(247,76,0);stop-opacity:1"/><stop offset="1" style="stop-color:rgb(247,76,0);stop-opacity:1"/></linearGradient>
                    <linearGradient id="_Linear3" x1="0" y1="0" x2="1" y2="0" gradientUnits="userSpaceOnUse" gradientTransform="matrix(1,1.32349e-23,1.32349e-23,-1,0,-9.1568e-07)"><stop offset="0" style="stop-color:rgb(204,58,0);stop-opacity:1"/><stop offset="0.15" style="stop-color:rgb(204,58,0);stop-opacity:1"/><stop offset="0.74" style="stop-color:rgb(247,76,0);stop-opacity:1"/><stop offset="1" style="stop-color:rgb(247,76,0);stop-opacity:1"/></linearGradient>
                </defs>
            </svg>"#;

        let dom = Dom::from_bytes(data.as_bytes(), FontMgr::default()).unwrap();
        let mut root = dom.root();

        println!("{:#?}", root.transform());
        println!("{:#?}", root.intrinsic_size());

        root.set_width(Length::new(50., LengthUnit::PX));
        root.set_height(Length::new(600., LengthUnit::CM));

        println!("{:#?}", root.intrinsic_size());
        println!("{:#?}", root.children_typed());
    }
}
