//! High-scores table UI via [`orbital::components::OrbitalInfiniteScroll`].
//!
//! [`HighScoresTable`] is the teaching client for paginated Valence data: pass
//! [`super::get_high_scores_page`] as the `fetch` closure, set `page_size` to
//! [`super::HIGH_SCORES_PAGE_SIZE`], and fill loading / empty / end slots with
//! Orbital primitives. Skeleton rows gate on hydrate so SSR markup stays stable.

use leptos::prelude::*;
use orbital::components::{
    Caption1, OrbitalInfiniteScroll, OrbitalInfiniteScrollEmptyView, OrbitalInfiniteScrollEndView,
    OrbitalInfiniteScrollLoadingView, SkeletonItemSize, TableCellLayoutConfig,
};
use orbital::primitives::{
    Flex, FlexGap, MessageBar, MessageBarIntent, Skeleton, SkeletonItem, Table, TableBody,
    TableCell, TableCellLayout, TableHeader, TableHeaderCell, TableRow,
};

use super::server::get_high_scores_page;
use super::types::{HighScoreEntry, HIGH_SCORES_PAGE_SIZE};

/// A single row in the high-scores leaderboard.
#[component]
fn HighScoreRow(
    /// Reactive rank index (0-based from `ForEnumerate`).
    idx: ReadSignal<usize>,
    /// The leaderboard entry to render.
    entry: HighScoreEntry,
) -> impl IntoView {
    let score = entry.count;
    let name = entry.display_name.clone();
    let name_attr = name.clone();
    view! {
        <TableRow>
            <TableCell>
                <div
                    data-testid="counter-high-score-row"
                    data-score=score.to_string()
                    data-name=name_attr
                >
                    <TableCellLayout>{move || idx.get() + 1}</TableCellLayout>
                </div>
            </TableCell>
            <TableCell>
                <TableCellLayout config=TableCellLayoutConfig { truncate: true }>{name}</TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>{score}</TableCellLayout>
            </TableCell>
        </TableRow>
    }
}

/// Tracks client hydration so loading-only UI does not diverge from SSR markup.
fn create_hydrated_signal() -> (ReadSignal<bool>, WriteSignal<bool>) {
    signal(false)
}

/// Skeleton rows only — headers live in the loaded table to avoid duplicate static text during hydration.
#[component]
fn HighScoresSkeletonRows() -> impl IntoView {
    let skeleton_size = Signal::from(SkeletonItemSize::S16);

    view! {
        <Skeleton>
            {(0..HIGH_SCORES_PAGE_SIZE)
                .map(|_| {
                    view! {
                        <TableRow>
                            <TableCell>
                                <TableCellLayout>
                                    <SkeletonItem size=skeleton_size />
                                </TableCellLayout>
                            </TableCell>
                            <TableCell>
                                <TableCellLayout>
                                    <SkeletonItem size=skeleton_size />
                                </TableCellLayout>
                            </TableCell>
                            <TableCell>
                                <TableCellLayout>
                                    <SkeletonItem size=skeleton_size />
                                </TableCellLayout>
                            </TableCell>
                        </TableRow>
                    }
                })
                .collect_view()}
        </Skeleton>
    }
}

/// Initial-load skeleton table. Gated until after hydration so SSR markup matches the first client pass.
#[component]
fn HighScoresTableSkeleton() -> impl IntoView {
    let (is_hydrated, set_hydrated) = create_hydrated_signal();

    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| set_hydrated.set(true));
    }

    view! {
        {move || {
            if is_hydrated.get() {
                view! {
                    <div data-testid="high-scores-skeleton">
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell>"Rank"</TableHeaderCell>
                                    <TableHeaderCell>"User"</TableHeaderCell>
                                    <TableHeaderCell>"Score"</TableHeaderCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                <HighScoresSkeletonRows />
                            </TableBody>
                        </Table>
                    </div>
                }
                .into_any()
            } else {
                ().into_any()
            }
        }}
    }
}

/// Paginated high-scores table driven by Orbital infinite scroll.
///
/// Mount under [`crate::counter::counter_example::HighScoresPage`]. Uses
/// [`OrbitalInfiniteScroll`] with `let:items` children for the table body and
/// loading / empty / end slots. The `fetch` closure calls
/// [`get_high_scores_page`] so each scroll window hits Valence through Higgs.
#[component]
pub fn HighScoresTable() -> impl IntoView {
    // Closures that contain generics or nested view! macros must be defined
    // outside the view! block — the macro parser cannot handle them inline.
    let fetch_scores = |offset: u32, limit: u32| get_high_scores_page(offset, limit);

    view! {
        // Thin testid + flex-child growth shell: Orbital `Flex::fill` is height:100%,
        // not `flex: 1`, so nested scroll hosts still need this growth escape.
        <div data-testid="high-scores" style="flex: 1; min-height: 0; width: 100%;">
            <Flex vertical=true fill=true full_width=true gap=FlexGap::Size(0)>
            <OrbitalInfiniteScroll
                page_size=HIGH_SCORES_PAGE_SIZE
                fetch=fetch_scores
                max_height="calc(100dvh - 220px)"
                let:items
            >
                <OrbitalInfiniteScrollLoadingView slot>
                    <HighScoresTableSkeleton />
                </OrbitalInfiniteScrollLoadingView>
                <OrbitalInfiniteScrollEmptyView slot>
                    <MessageBar intent=MessageBarIntent::Info>
                        "No scores yet."
                    </MessageBar>
                </OrbitalInfiniteScrollEmptyView>
                <OrbitalInfiniteScrollEndView slot>
                    <Caption1>"End of leaderboard"</Caption1>
                </OrbitalInfiniteScrollEndView>
                <Table>
                    <TableHeader>
                        <TableRow>
                            <TableHeaderCell>"Rank"</TableHeaderCell>
                            <TableHeaderCell>"User"</TableHeaderCell>
                            <TableHeaderCell>"Score"</TableHeaderCell>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        <ForEnumerate
                            each=move || items.get()
                            key=|entry| {
                                if entry.row_key.is_empty() {
                                    format!("{}:{}", entry.display_name, entry.count)
                                } else {
                                    entry.row_key.clone()
                                }
                            }
                            let(idx, entry)
                        >
                            <HighScoreRow idx=idx entry=entry />
                        </ForEnumerate>
                    </TableBody>
                </Table>
            </OrbitalInfiniteScroll>
            </Flex>
        </div>
    }
}
