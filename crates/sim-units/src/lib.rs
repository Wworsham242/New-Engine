#![forbid(unsafe_code)]

macro_rules! scalar_unit {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        pub struct $name(pub f64);

        impl $name {
            pub fn nonnegative(self) -> Self {
                Self(self.0.max(0.0))
            }

            pub fn is_finite(self) -> bool {
                self.0.is_finite()
            }
        }
    };
}

scalar_unit!(RealGdp);
scalar_unit!(Currency);
scalar_unit!(EnergyQuantity);
scalar_unit!(PriceIndex);
scalar_unit!(Rate);
scalar_unit!(Population);
