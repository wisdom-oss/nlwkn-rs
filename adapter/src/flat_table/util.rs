use std::format;
use std::marker::PhantomData;

use nlwkn::helper_types::OrFallback;
use nlwkn::{LandRecord, LegalDepartment, RateRecord, UsageLocation, WaterRight};

use crate::flat_table::key::{marker, FlatTableKey};
use crate::flat_table::value::FlatTableValue;
use crate::flat_table::{FlatTableRow, FlatTableRows};

pub fn insert_into_row<M, V>(
    row: &mut FlatTableRow<M>,
    key: FlatTableKey<marker::Unselect>,
    value: Option<V>
) where
    V: Into<FlatTableValue>,
    FlatTableKey<M>: AsRef<str>
{
    if let Some(value) = value {
        row.insert(FlatTableKey::from_unselect(key), value.into());
    }
}

pub fn insert_rate_record_into_row<M>(
    row: &mut FlatTableRow<M>,
    key: FlatTableKey<marker::Unselect>,
    rate_record: &RateRecord
) where
    FlatTableKey<M>: AsRef<str>
{
    for rate in rate_record.iter().filter_map(|item| match item {
        OrFallback::Fallback(_) => None,
        OrFallback::Expected(rate) => Some(rate)
    }) {
        let key: FlatTableKey<M> = FlatTableKey::Multiple {
            phantom: PhantomData,
            de: format!("{}/{}", key.ref_de(), rate.per).into(),
            en: format!("{}/{}", key.ref_en(), rate.per).into()
        };

        row.insert(key, format!("{} {}", rate.value, rate.unit).into());
    }
}

pub fn flatten_water_right<M>(water_right: &WaterRight) -> FlatTableRows<M>
where
    FlatTableKey<M>: AsRef<str>
{
    let mut rows = FlatTableRows::new();
    for ld in water_right.legal_departments.values() {
        rows.append(&mut flatten_legal_department(ld));
    }

    for mut row in rows.iter_mut() {
        // destructure the water right to make sure every field of it is used
        #[deny(unused_variables)]
        let WaterRight {
            no,
            rights_holder,
            valid_until,
            status,
            valid_from,
            legal_title,
            water_authority,
            registering_authority,
            granting_authority,
            first_grant,
            date_of_change,
            file_reference,
            external_identifier,
            subject,
            address,
            annotation,
            legal_departments: _
        } = water_right;

        insert_into_row(&mut row, FlatTableKey::NO, Some(no.clone()));
        insert_into_row(&mut row, FlatTableKey::RIGHTS_HOLDER, rights_holder.clone());
        insert_into_row(&mut row, FlatTableKey::VALID_UNTIL, valid_until.clone());
        insert_into_row(&mut row, FlatTableKey::STATUS, status.clone());
        insert_into_row(&mut row, FlatTableKey::VALID_FROM, valid_from.clone());
        insert_into_row(&mut row, FlatTableKey::LEGAL_TITLE, legal_title.clone());
        insert_into_row(
            &mut row,
            FlatTableKey::WATER_AUTHORITY,
            water_authority.clone()
        );
        insert_into_row(
            &mut row,
            FlatTableKey::REGISTERING_AUTHORITY,
            registering_authority.clone()
        );
        insert_into_row(
            &mut row,
            FlatTableKey::GRANTING_AUTHORITY,
            granting_authority.clone()
        );
        insert_into_row(&mut row, FlatTableKey::FIRST_GRANT, first_grant.clone());
        insert_into_row(
            &mut row,
            FlatTableKey::DATE_OF_CHANGE,
            date_of_change.clone()
        );
        insert_into_row(
            &mut row,
            FlatTableKey::FILE_REFERENCE,
            file_reference.clone()
        );
        insert_into_row(
            &mut row,
            FlatTableKey::EXTERNAL_IDENTIFIER,
            external_identifier.clone()
        );
        insert_into_row(&mut row, FlatTableKey::SUBJECT, subject.clone());
        insert_into_row(&mut row, FlatTableKey::ADDRESS, address.clone());
        insert_into_row(&mut row, FlatTableKey::ANNOTATION, annotation.clone());
    }

    rows
}

fn flatten_legal_department<M>(legal_department: &LegalDepartment) -> FlatTableRows<M>
where
    FlatTableKey<M>: AsRef<str>
{
    // destructure the legal department to make sure every field of it is used
    #[deny(unused_variables)]
    let LegalDepartment {
        usage_locations,
        description,
        abbreviation
    } = legal_department;

    let mut rows = FlatTableRows::new();
    for usage_location in usage_locations.iter() {
        let mut row = flatten_usage_location(usage_location);
        insert_into_row(
            &mut row,
            FlatTableKey::LEGAL_DEPARTMENT_DESCRIPTION,
            Some(description.clone())
        );
        insert_into_row(
            &mut row,
            FlatTableKey::LEGAL_DEPARTMENT_ABBREVIATION,
            Some(abbreviation.to_string())
        );
        rows.push(row);
    }

    rows
}

fn flatten_usage_location<M>(usage_location: &UsageLocation) -> FlatTableRow<M>
where
    FlatTableKey<M>: AsRef<str>
{
    // destructure usage location to make sure every field is used
    #[deny(unused_variables)]
    let UsageLocation {
        no,
        serial_no,
        active,
        real,
        name,
        legal_purpose,
        top_map_1_25000,
        municipal_area,
        county,
        land_record,
        plot,
        maintenance_association,
        eu_survey_area,
        basin_code,
        regulation_citation,
        withdrawal_rate,
        pumping_rate,
        injection_rate,
        waste_water_flow_volume,
        river_basin,
        groundwater_body,
        water_body,
        flood_area,
        water_protection_area,
        dam_target_levels,
        fluid_discharge,
        rain_supplement,
        irrigation_area,
        ph_values,
        injection_limit,
        utm_easting,
        utm_northing
    } = usage_location;

    let mut row = FlatTableRow::new();
    insert_into_row(&mut row, FlatTableKey::USAGE_LOCATION_NO, no.clone());
    insert_into_row(
        &mut row,
        FlatTableKey::USAGE_LOCATION_SERIAL_NO,
        serial_no.clone()
    );
    insert_into_row(&mut row, FlatTableKey::ACTIVE, active.clone());
    insert_into_row(&mut row, FlatTableKey::REAL, real.clone());
    insert_into_row(&mut row, FlatTableKey::USAGE_LOCATION_NAME, name.clone());
    insert_into_row(
        &mut row,
        FlatTableKey::LEGAL_PURPOSE,
        legal_purpose.as_ref().map(|(code, name)| format!("{code} {name}"))
    );
    insert_into_row(
        &mut row,
        FlatTableKey::TOP_MAP_1_25000,
        top_map_1_25000.as_ref().map(ToString::to_string)
    );
    insert_into_row(
        &mut row,
        FlatTableKey::MUNICIPAL_AREA,
        municipal_area.as_ref().map(|(code, name)| format!("{code} {name}"))
    );
    insert_into_row(&mut row, FlatTableKey::COUNTY, county.clone());

    match land_record.as_ref() {
        None => (),
        Some(OrFallback::Fallback(s)) => {
            insert_into_row(&mut row, FlatTableKey::LAND_RECORD, Some(s.clone()))
        }
        Some(OrFallback::Expected(LandRecord {
            register_district,
            field_number
        })) => insert_into_row(
            &mut row,
            FlatTableKey::LAND_RECORD,
            Some(format!("{register_district}{field_number}"))
        )
    }

    insert_into_row(&mut row, FlatTableKey::PLOT, plot.clone());
    insert_into_row(
        &mut row,
        FlatTableKey::MAINTENANCE_ASSOCIATION,
        maintenance_association.as_ref().map(|(code, name)| format!("{code} {name}"))
    );
    insert_into_row(
        &mut row,
        FlatTableKey::EU_SURVEY_AREA,
        eu_survey_area.as_ref().map(|(code, name)| format!("{code} {name}"))
    );
    insert_into_row(
        &mut row,
        FlatTableKey::BASIN_CODE,
        basin_code.as_ref().map(ToString::to_string)
    );
    insert_into_row(
        &mut row,
        FlatTableKey::REGULATION_CITATION,
        regulation_citation.clone()
    );
    insert_rate_record_into_row(&mut row, FlatTableKey::WITHDRAWAL_RATE, withdrawal_rate);
    insert_rate_record_into_row(&mut row, FlatTableKey::PUMPING_RATE, pumping_rate);
    insert_rate_record_into_row(&mut row, FlatTableKey::INJECTION_RATE, injection_rate);
    insert_rate_record_into_row(
        &mut row,
        FlatTableKey::WASTER_WATER_FLOW_VOLUME,
        waste_water_flow_volume
    );
    insert_into_row(&mut row, FlatTableKey::RIVER_BASIN, river_basin.clone());
    insert_into_row(
        &mut row,
        FlatTableKey::GROUNDWATER_BODY,
        groundwater_body.clone()
    );
    insert_into_row(&mut row, FlatTableKey::WATER_BODY, water_body.clone());
    insert_into_row(&mut row, FlatTableKey::FLOOD_AREA, flood_area.clone());
    insert_into_row(
        &mut row,
        FlatTableKey::WATER_PROTECTION_AREA,
        water_protection_area.clone()
    );
    insert_into_row(
        &mut row,
        FlatTableKey::DAM_TARGETS_DEFAULT,
        dam_target_levels.default.as_ref().map(ToString::to_string)
    );
    insert_into_row(
        &mut row,
        FlatTableKey::DAM_TARGETS_STEADY,
        dam_target_levels.steady.as_ref().map(ToString::to_string)
    );
    insert_into_row(
        &mut row,
        FlatTableKey::DAM_TARGETS_MAX,
        dam_target_levels.max.as_ref().map(ToString::to_string)
    );
    insert_rate_record_into_row(&mut row, FlatTableKey::FLUID_DISCHARGE, fluid_discharge);
    insert_rate_record_into_row(&mut row, FlatTableKey::RAIN_SUPPLEMENT, rain_supplement);
    insert_into_row(
        &mut row,
        FlatTableKey::IRRIGATION_AREA,
        irrigation_area.as_ref().map(ToString::to_string)
    );
    insert_into_row(
        &mut row,
        FlatTableKey::PH_VALUES_MIN,
        ph_values.as_ref().map(|v| v.min).flatten()
    );
    insert_into_row(
        &mut row,
        FlatTableKey::PH_VALUES_MAX,
        ph_values.as_ref().map(|v| v.max).flatten()
    );

    for (key, quantity) in injection_limit.iter() {
        row.insert(FlatTableKey::from(key.clone()), quantity.to_string().into());
    }

    insert_into_row(&mut row, FlatTableKey::UTM_EASTING, utm_easting.clone());
    insert_into_row(&mut row, FlatTableKey::UTM_NORTHING, utm_northing.clone());

    row
}
