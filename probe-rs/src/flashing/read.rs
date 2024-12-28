use std::collections::HashMap;

use probe_rs_target::{MemoryRange, MemoryRegion, NvmRegion};

use crate::flashing::{flasher::Flasher, FlashError, FlashLoader};
use crate::flashing::{FlashAlgorithm, FlashLayout, FlashSector};
use crate::Session;

use super::FlashProgress;

/// Reads `data.len()` bytes from `address`.
pub fn read(
    session: &mut Session,
    progress: FlashProgress,
    address: u64,
    data: &mut [u8],
) -> Result<(), FlashError> {

    tracing::warn!("Read {} bytes from {:#x?}", data.len(), address);

    let mut algos: HashMap<(String, String), Vec<NvmRegion>> = HashMap::new();

    for region in session
        .target()
        .memory_map
        .iter()
        .filter_map(MemoryRegion::as_nvm_region)
    {
        if region.is_alias {
            tracing::debug!("Skipping alias memory region {:#010x?}", region.range);
            continue;
        }
        tracing::debug!(
            "    region: {:#010x?} ({} bytes)",
            region.range,
            region.range.end - region.range.start
        );

        let algo = FlashLoader::get_flash_algorithm_for_region(region, session.target())?;

        // Get the first core that can access the region
        let core_name = region
            .cores
            .first()
            .ok_or_else(|| FlashError::NoNvmCoreAccess(region.clone()))?;

        let entry = algos
            .entry((algo.name.clone(), core_name.clone()))
            .or_default();
        entry.push(region.clone());

        tracing::debug!("     -- using algorithm: {}", algo.name);
    }

    for ((algo_name, core_name), _regions) in algos {
        tracing::debug!("Reading with algorithm: {}", algo_name);

        // This can't fail, algo_name comes from the target.
        let algo = session.target().flash_algorithm_by_name(&algo_name);
        let algo = algo.unwrap().clone();

        // This can't fail, algo_name comes from the target.
        let core_index = session.target().core_index_by_name(&core_name).unwrap();
        let mut flasher = Flasher::new(session, core_index, &algo, progress.clone())?;

        flasher.run_verify(|active| {
            active.read_flash(address, data)?;
            Ok(true)
        })?;
    }

    Ok(())
}
