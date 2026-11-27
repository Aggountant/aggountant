//! Mileage allowance (kilometrikorvaus), decision 9 §.
//!
//! A drive's edition follows the drive's date. The car surcharges apply to
//! every kilometre of the drive, each at most once whatever the load
//! (guidance 8.2.4); the forest-road surcharge only to the kilometres driven
//! on such a road; the passenger surcharge to every vehicle, per passenger. A
//! car benefit car gets the fuel rate only, without surcharges.

use rust_decimal::Decimal;

use super::{money, Context, Drive, DriveComputation, Surcharge, Vehicle};
use crate::DomainError;

pub(super) struct Mileage {
    pub amount: Decimal,
    pub km: Decimal,
    pub drives: Vec<DriveComputation>,
}

/// Compute the drives of trip `trip`, which [`validate`] has checked against
/// the trip's Finnish start and end dates (`trip::check`).
pub(super) fn compute(trip: usize, drives: &[Drive], ctx: &mut Context) -> Mileage {
    let mut out = Mileage {
        amount: Decimal::ZERO,
        km: Decimal::ZERO,
        drives: Vec::with_capacity(drives.len()),
    };
    for drive in drives {
        let year = drive.date.inner().year();
        let rate = ctx.edition(year, trip, None).map(|edition| {
            if drive.car_benefit {
                return (edition.car_benefit_fuel_eur_per_km, Decimal::ZERO);
            }
            let v = &edition.mileage_eur_per_km;
            let base = match drive.vehicle {
                Vehicle::Car => v.car,
                Vehicle::MotorboatUpTo50hp => v.motorboat_up_to_50hp,
                Vehicle::MotorboatOver50hp => v.motorboat_over_50hp,
                Vehicle::Snowmobile => v.snowmobile,
                Vehicle::Atv => v.atv,
                Vehicle::Motorcycle => v.motorcycle,
                Vehicle::Moped => v.moped,
                Vehicle::Other => v.other,
            };
            let s = &edition.car_surcharges_eur_per_km;
            let surcharges: Decimal = drive
                .surcharges
                .iter()
                .map(|x| match x {
                    Surcharge::Trailer => s.trailer,
                    Surcharge::Caravan => s.caravan,
                    Surcharge::HeavyLoad => s.heavy_load,
                    Surcharge::HeavyEquipment => s.heavy_equipment,
                    Surcharge::Dog => s.dog,
                })
                .sum();
            let passengers =
                edition.passenger_surcharge_eur_per_km * Decimal::from(drive.passengers);
            (base + surcharges + passengers, s.forest_road)
        });
        let amount = rate.map(|(per_km, forest_per_km)| {
            drive.km * per_km + drive.forest_road_km.unwrap_or_default() * forest_per_km
        });
        if let Some(a) = amount {
            out.amount += a;
        }
        out.km += drive.km;
        out.drives.push(DriveComputation {
            date: drive.date.to_string(),
            km: drive.km.normalize().to_string(),
            vehicle: drive.vehicle,
            rate_eur_per_km: rate.map(|(r, _)| money(r)),
            amount_exact: amount.map(money),
        });
    }
    out
}

pub(super) fn validate(
    trip: usize,
    n: usize,
    drive: &Drive,
    first_date: jiff::civil::Date,
    last_date: jiff::civil::Date,
) -> Result<(), DomainError> {
    let bad = |what: String| {
        Err(DomainError::TravelInput(format!(
            "trip {trip}, drive {n}: {what}"
        )))
    };
    let date = drive.date.inner();
    if date < first_date || date > last_date {
        return bad(format!(
            "date {} is outside the trip ({first_date}..={last_date} in Finnish time)",
            drive.date
        ));
    }
    if drive.km <= Decimal::ZERO {
        return bad("km must be positive".into());
    }
    let mut seen = drive.surcharges.clone();
    seen.sort();
    seen.dedup();
    if seen.len() != drive.surcharges.len() {
        return bad("a surcharge is listed twice; each applies at most once per kilometre".into());
    }
    let car_only = !drive.surcharges.is_empty() || drive.forest_road_km.is_some();
    if car_only && drive.vehicle != Vehicle::Car {
        return bad("surcharges and forest_road_km apply to a car only".into());
    }
    if let Some(forest) = drive.forest_road_km {
        if forest <= Decimal::ZERO || forest > drive.km {
            return bad("forest_road_km must be positive and at most km".into());
        }
    }
    if drive.car_benefit && (drive.vehicle != Vehicle::Car || car_only || drive.passengers > 0) {
        return bad(
            "a car benefit drive is a car with the fuel rate only: no surcharges, forest road or passengers".into(),
        );
    }
    Ok(())
}
