use super::{
    checkpoint::{Checkpoint, MappingSerialization},
    page_allocator::PageAllocatorSerialization,
    Buffer, FileDescriptor, PageAllocatorRegistry, PageIndex, PageMap, PageMapSerialization,
};
use crate::page_map::{MemoryRegion, TestPageAllocatorFileDescriptorImpl, WRITE_BUCKET_PAGES};
use ic_sys::PAGE_SIZE;
use ic_types::{Height, MAX_STABLE_MEMORY_IN_BYTES};
use nix::unistd::dup;
use static_assertions::const_assert_ne;
use std::sync::Arc;
use std::{fs::OpenOptions, ops::Range, path::Path};

fn assert_equal_page_maps(page_map1: &PageMap, page_map2: &PageMap) {
    assert_eq!(page_map1.num_host_pages(), page_map2.num_host_pages());
    for i in 0..page_map1.num_host_pages() {
        assert_eq!(
            page_map1.get_page(PageIndex::new(i as u64)),
            page_map2.get_page(PageIndex::new(i as u64))
        );
    }
}

// Since tests run in the same process, we need to duplicate all file
// descriptors so that both page maps can close them.
fn duplicate_file_descriptors(
    mut serialized_page_map: PageMapSerialization,
) -> PageMapSerialization {
    serialized_page_map.checkpoint.mapping =
        serialized_page_map
            .checkpoint
            .mapping
            .map(|mapping| MappingSerialization {
                file_descriptor: FileDescriptor {
                    fd: dup(mapping.file_descriptor.fd).unwrap(),
                },
                ..mapping
            });
    serialized_page_map.page_allocator = PageAllocatorSerialization {
        id: serialized_page_map.page_allocator.id,
        fd: FileDescriptor {
            fd: dup(serialized_page_map.page_allocator.fd.fd).unwrap(),
        },
    };
    serialized_page_map
}

#[test]
fn can_debug_display_a_page_map() {
    let page_map = PageMap::new_for_testing();
    assert_eq!(format!("{:?}", page_map), "{}");
}

#[test]
fn can_create_an_empty_checkpoint() {
    let checkpoint = Checkpoint::empty();
    let empty_page = vec![0; PAGE_SIZE];
    let first_page = checkpoint.get_page(PageIndex::new(1));
    assert_eq!(&empty_page[..], first_page);
}

#[test]
fn empty_page_map_returns_zeroed_pages() {
    let page_map = PageMap::new_for_testing();
    let page = page_map.get_page(PageIndex::new(1));
    assert_eq!(page.len(), PAGE_SIZE);
    assert!(page.iter().all(|b| *b == 0));
}

#[test]
fn can_update_a_page_map() {
    let mut page_map = PageMap::new_for_testing();
    let ones = [1u8; PAGE_SIZE];
    let twos = [2u8; PAGE_SIZE];

    let delta = [(PageIndex::new(1), &ones), (PageIndex::new(2), &twos)];

    page_map.update(&delta);

    for (num, contents) in &[(1, 1), (2, 2), (3, 0)] {
        assert!(page_map
            .get_page(PageIndex::new(*num))
            .iter()
            .all(|b| *b == *contents));
    }
}

#[test]
fn new_delta_wins_on_update() {
    let mut page_map = PageMap::new_for_testing();
    let page_1 = [1u8; PAGE_SIZE];
    let page_2 = [2u8; PAGE_SIZE];

    let pages_1 = &[(PageIndex::new(1), &page_1)];
    let pages_2 = &[(PageIndex::new(1), &page_2)];

    page_map.update(pages_1);
    page_map.update(pages_2);

    assert_eq!(page_map.get_page(PageIndex::new(1)), &page_2);
}

#[test]
fn persisted_map_is_equivalent_to_the_original() {
    fn persist_check_eq_and_load(
        pagemap: &mut PageMap,
        heap_file: &Path,
        pages_to_update: &[(PageIndex, [u8; PAGE_SIZE])],
    ) -> PageMap {
        pagemap.update(
            &pages_to_update
                .iter()
                .map(|(idx, p)| (*idx, p))
                .collect::<Vec<_>>(),
        );
        pagemap.persist_delta(heap_file).unwrap();
        let persisted_map = PageMap::open(
            heap_file,
            Height::new(0),
            Arc::new(TestPageAllocatorFileDescriptorImpl::new()),
        )
        .unwrap();

        assert_eq!(*pagemap, persisted_map);
        persisted_map
    }

    let tmp = tempfile::Builder::new()
        .prefix("checkpoints")
        .tempdir()
        .unwrap();
    let heap_file = tmp.path().join("heap");

    let base_page = [42u8; PAGE_SIZE];
    let base_data = vec![&base_page; 50];
    let mut pagemap = persist_check_eq_and_load(
        &mut PageMap::new_for_testing(),
        &heap_file,
        &base_data
            .iter()
            .enumerate()
            .map(|(i, page)| (PageIndex::new(i as u64), **page))
            .collect::<Vec<_>>(),
    );

    let mut pagemap = persist_check_eq_and_load(
        &mut pagemap,
        &heap_file,
        &[
            (PageIndex::new(1), [1u8; PAGE_SIZE]),
            (PageIndex::new(3), [3u8; PAGE_SIZE]),
            (PageIndex::new(4), [4u8; PAGE_SIZE]),
            (PageIndex::new(60), [60u8; PAGE_SIZE]),
            (PageIndex::new(63), [63u8; PAGE_SIZE]),
            (PageIndex::new(100), [100u8; PAGE_SIZE]),
        ],
    );

    let mut pagemap = persist_check_eq_and_load(
        &mut pagemap,
        &heap_file,
        &[(PageIndex::new(1), [255u8; PAGE_SIZE])],
    );
    // Check that it's possible to serialize without reloading.
    persist_check_eq_and_load(
        &mut pagemap,
        &heap_file,
        &[(PageIndex::new(104), [104u8; PAGE_SIZE])],
    );
    persist_check_eq_and_load(
        &mut pagemap,
        &heap_file,
        &[(PageIndex::new(103), [103u8; PAGE_SIZE])],
    );
    assert_eq!(105 * PAGE_SIZE as u64, heap_file.metadata().unwrap().len());
}

#[test]
fn can_persist_and_load_an_empty_page_map() {
    let tmp = tempfile::Builder::new()
        .prefix("checkpoints")
        .tempdir()
        .unwrap();
    let heap_file = tmp.path().join("heap");

    let original_map = PageMap::new_for_testing();
    original_map.persist_delta(&heap_file).unwrap();
    let persisted_map = PageMap::open(
        &heap_file,
        Height::new(0),
        Arc::new(TestPageAllocatorFileDescriptorImpl::new()),
    )
    .expect("opening an empty page map must succeed");

    // base_height will be different, but is not part of eq
    assert_eq!(original_map, persisted_map);
}

#[test]
fn returns_an_error_if_file_size_is_not_a_multiple_of_page_size() {
    use std::io::Write;

    let tmp = tempfile::Builder::new()
        .prefix("checkpoints")
        .tempdir()
        .unwrap();
    let heap_file = tmp.path().join("heap");
    OpenOptions::new()
        .write(true)
        .create(true)
        .open(&heap_file)
        .unwrap()
        .write_all(&vec![1; PAGE_SIZE / 2])
        .unwrap();

    match PageMap::open(
        &heap_file,
        Height::new(0),
        Arc::new(TestPageAllocatorFileDescriptorImpl::new()),
    ) {
        Err(err) => assert!(
            err.is_invalid_heap_file(),
            "Expected invalid heap file error, got {:?}",
            err
        ),
        Ok(_) => panic!("Expected a invalid heap file error, got Ok(_)"),
    }
}

#[test]
fn can_use_buffer_to_modify_page_map() {
    let page_1 = [1u8; PAGE_SIZE];
    let page_3 = [3u8; PAGE_SIZE];
    let pages = &[(PageIndex::new(1), &page_1), (PageIndex::new(3), &page_3)];
    let mut page_map = PageMap::new_for_testing();
    page_map.update(pages);

    let n = 4 * PAGE_SIZE;
    let mut vec_buf = vec![0u8; n];
    vec_buf[PAGE_SIZE..2 * PAGE_SIZE].copy_from_slice(&page_1);
    vec_buf[3 * PAGE_SIZE..4 * PAGE_SIZE].copy_from_slice(&page_3);

    let mut buf = Buffer::new(page_map);

    let mut read_buf = vec![0u8; n];

    buf.read(&mut read_buf[..], 0);
    assert_eq!(read_buf, vec_buf);

    for offset in 0..n {
        let mut len = 1;
        while (offset + len) < n {
            let b = ((offset + len) % 15) as u8;
            for dst in vec_buf.iter_mut().skip(offset).take(len) {
                *dst = b;
            }
            buf.write(&vec_buf[offset..offset + len], offset);
            buf.read(&mut read_buf[..], 0);
            assert_eq!(read_buf, vec_buf);
            len *= 2;
        }
    }
}

#[test]
fn serialize_empty_page_map() {
    let page_allocator_registry = PageAllocatorRegistry::new();
    let original_page_map = PageMap::new_for_testing();
    let serialized_page_map = duplicate_file_descriptors(original_page_map.serialize());
    let deserialized_page_map =
        PageMap::deserialize(serialized_page_map, &page_allocator_registry).unwrap();
    assert_equal_page_maps(&original_page_map, &deserialized_page_map);
}

#[test]
fn serialize_page_map() {
    let page_allocator_registry = PageAllocatorRegistry::new();
    let mut replica = PageMap::new_for_testing();
    // The replica process sends the page map to the sandbox process.
    let serialized_page_map = duplicate_file_descriptors(replica.serialize());
    let mut sandbox = PageMap::deserialize(serialized_page_map, &page_allocator_registry).unwrap();
    // The sandbox process allocates new pages.
    let page_1 = [1u8; PAGE_SIZE];
    let page_3 = [3u8; PAGE_SIZE];
    let page_7 = [7u8; PAGE_SIZE];
    let pages = &[(PageIndex::new(1), &page_1), (PageIndex::new(3), &page_3)];
    sandbox.update(pages);
    sandbox.strip_unflushed_delta();
    sandbox.update(&[(PageIndex::new(7), &page_7)]);
    // The sandbox process sends the dirty pages to the replica process.
    let page_delta =
        sandbox.serialize_delta(&[PageIndex::new(1), PageIndex::new(3), PageIndex::new(7)]);
    replica.deserialize_delta(page_delta);
    // The page deltas must be in sync.
    assert_equal_page_maps(&replica, &sandbox);
}

/// Check that the value provided by `calculate_dirty_pages` agrees with the
/// actual change in number of dirty pages and return the number of new dirty
/// pages.
fn write_and_verify_dirty_pages(buf: &mut Buffer, src: &[u8], offset: usize) -> u64 {
    let new = buf.dirty_pages_from_write(offset as u64, src.len() as u64);
    let initial = buf.dirty_pages.len();
    buf.write(src, offset);
    let updated = buf.dirty_pages.len();
    assert_eq!(updated - initial, new.get() as usize);
    new.get()
}

/// Complete re-write of first page is dirty, later write doesn't increase
/// count.
#[test]
fn buffer_entire_first_page_write() {
    let mut buf = Buffer::new(PageMap::new_for_testing());
    assert_eq!(
        1,
        write_and_verify_dirty_pages(&mut buf, &[0; PAGE_SIZE], 0)
    );
    assert_eq!(0, write_and_verify_dirty_pages(&mut buf, &[0; 1], 0));
}

/// Single write to first page is dirty, later write doesn't increase count.
#[test]
fn buffer_single_byte_first_page_write() {
    let mut buf = Buffer::new(PageMap::new_for_testing());
    assert_eq!(1, write_and_verify_dirty_pages(&mut buf, &[0; 1], 0));
    assert_eq!(0, write_and_verify_dirty_pages(&mut buf, &[0; 1], 1));
}

#[test]
fn buffer_write_single_byte_each_page() {
    let mut buf = Buffer::new(PageMap::new_for_testing());
    assert_eq!(1, write_and_verify_dirty_pages(&mut buf, &[0; 1], 0));
    assert_eq!(
        1,
        write_and_verify_dirty_pages(&mut buf, &[0; 1], PAGE_SIZE)
    );
    assert_eq!(
        1,
        write_and_verify_dirty_pages(&mut buf, &[0; 1], 2 * PAGE_SIZE)
    );
    assert_eq!(
        1,
        write_and_verify_dirty_pages(&mut buf, &[0; 1], 15 * PAGE_SIZE)
    );
}

#[test]
fn buffer_write_unaligned_multiple_pages() {
    const NUM_PAGES: u64 = 3;
    let mut buf = Buffer::new(PageMap::new_for_testing());
    assert_eq!(
        NUM_PAGES + 1,
        write_and_verify_dirty_pages(&mut buf, &[0; (NUM_PAGES as usize) * PAGE_SIZE], 24)
    );
}

#[test]
fn buffer_write_empty_slice() {
    let mut buf = Buffer::new(PageMap::new_for_testing());
    assert_eq!(0, write_and_verify_dirty_pages(&mut buf, &[0; 0], 10_000));
}

// Checks that the pre-computed dirty pages agrees with the difference in dirty
// pages from before and after a write.
#[test]
fn calc_dirty_pages_matches_actual_change() {
    let mut runner = proptest::test_runner::TestRunner::deterministic();
    runner
        .run(
            &(0..MAX_STABLE_MEMORY_IN_BYTES, 0..(1000 * PAGE_SIZE as u64)),
            |(offset, size)| {
                // bound size to valid range
                let size = (MAX_STABLE_MEMORY_IN_BYTES - offset).min(size);
                let src = vec![0; size as usize];
                // Start with a buffer that has some initial dirty pages
                let mut buffer = Buffer::new(PageMap::new_for_testing());
                buffer.write(&[1; 10 * PAGE_SIZE], 5 * PAGE_SIZE + 10);
                buffer.write(&[3; 16], 44 * PAGE_SIZE);

                write_and_verify_dirty_pages(&mut buffer, &src, offset as usize);
                Ok(())
            },
        )
        .unwrap()
}

#[test]
fn zeros_region_after_delta() {
    let mut page_map = PageMap::new_for_testing();
    let tmp = tempfile::Builder::new()
        .prefix("checkpoints")
        .tempdir()
        .unwrap();
    let heap_file = tmp.path().join("heap");
    let pages = &[(PageIndex::new(1), &[1u8; PAGE_SIZE])];
    page_map.update(pages);

    page_map.persist_delta(&heap_file).unwrap();

    let mut page_map = PageMap::open(
        &heap_file,
        Height::new(0),
        Arc::new(TestPageAllocatorFileDescriptorImpl::new()),
    )
    .unwrap();

    let zero_range = page_map.get_memory_region(PageIndex::new(6));
    assert_eq!(
        MemoryRegion::Zeros(Range {
            start: PageIndex::new(2),
            end: PageIndex::new(u64::MAX)
        }),
        zero_range
    );

    // Add a page that is not an end of the bucket.
    const_assert_ne!((4 + 1) % WRITE_BUCKET_PAGES, 0);
    let pages = &[(PageIndex::new(4), &[1u8; PAGE_SIZE])];
    page_map.update(pages);

    let zero_range = page_map.get_memory_region(PageIndex::new(6));
    assert_eq!(
        MemoryRegion::Zeros(Range {
            start: PageIndex::new(5),
            end: PageIndex::new(u64::MAX)
        }),
        zero_range
    );

    // No trailing zero pages are serialized.
    page_map.persist_delta(&heap_file).unwrap();
    assert_eq!(5 * PAGE_SIZE as u64, heap_file.metadata().unwrap().len());
}

#[test]
fn zeros_region_within_delta() {
    let mut page_map = PageMap::new_for_testing();
    let tmp = tempfile::Builder::new()
        .prefix("checkpoints")
        .tempdir()
        .unwrap();
    let heap_file = tmp.path().join("heap");
    let pages = &[(PageIndex::new(1), &[1u8; PAGE_SIZE])];
    page_map.update(pages);

    page_map.persist_delta(&heap_file).unwrap();

    let mut page_map = PageMap::open(
        &heap_file,
        Height::new(0),
        Arc::new(TestPageAllocatorFileDescriptorImpl::new()),
    )
    .unwrap();

    let zero_range = page_map.get_memory_region(PageIndex::new(5));
    assert_eq!(
        MemoryRegion::Zeros(Range {
            start: PageIndex::new(2),
            end: PageIndex::new(u64::MAX)
        }),
        zero_range
    );

    let pages = &[
        (PageIndex::new(3), &[1u8; PAGE_SIZE]),
        (PageIndex::new(10), &[1u8; PAGE_SIZE]),
    ];
    page_map.update(pages);

    let zero_range = page_map.get_memory_region(PageIndex::new(5));
    assert_eq!(
        MemoryRegion::Zeros(Range {
            start: PageIndex::new(4),
            end: PageIndex::new(10)
        }),
        zero_range
    );
}
