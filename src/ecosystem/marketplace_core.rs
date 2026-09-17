// Amber 插件市场平台核心模块
// Stage 86 Phase 3 - 插件市场平台实现
//
// 提供插件搜索、发现、评分、评论和管理的完整功能

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex, atomic::Ordering};
use std::time::Instant;
use std::sync::atomic::Ordering;
use std::hash::{Hash, Hasher, DefaultHasher};

/// 插件市场主引擎
#[derive(Debug, Clone)]
pub struct PluginMarketplace {
    /// 插件索引数据库
    plugin_index: Arc<PluginIndex>,
    /// 搜索引擎
    search_engine: Arc<SearchEngine>,
    /// 评分系统
    rating_system: Arc<RatingSystem>,
    /// 审核系统
    review_system: Arc<ReviewSystem>,
    /// 缓存层
    cache: Arc<MarketplaceCache>,
}
/// 插件索引
#[derive(Debug, Clone)]
pub struct PluginIndex {
    /// 插件元数据存储
    plugins: HashMap<PluginId, PluginMetadata>,
    /// 标签索引
    tag_index: BTreeMap<String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>,
    /// 作者索引
    author_index: BTreeMap<String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>,
    /// 分类索引
    category_index: BTreeMap<String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId, String, HashSet<PluginId>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>,
}
/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub plugin_id: PluginId,
    pub name: String,
    pub description: String,
    pub long_description: Option<String>,
    pub author: PluginAuthor,
    pub version: Version,
    pub amberjs_version: VersionConstraint,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub icon_url: Option<String>,
    pub screenshots: Vec<String>,
    pub downloads: DownloadStats,
    pub rating: PluginRating,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub verified: bool,
    pub featured: bool,
    pub status: PluginStatus,
}
/// 插件作者
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
    pub avatar_url: Option<String>,
}
/// 插件 ID
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PluginId {
    pub name: String,
    pub version: Version,
}
/// 插件评分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRating {
    pub average: f64,
    pub count: u64,
    pub distribution: RatingDistribution,
}
/// 评分分布
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingDistribution {
    pub five_star: u64,
    pub four_star: u64,
    pub three_star: u64,
    pub two_star: u64,
    pub one_star: u64,
}
/// 下载统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStats {
    pub total: u64,
    pub recent: u64,
    pub trend: DownloadTrend,
}
/// 下载趋势
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTrend {
    pub daily: Vec<u64>,
    pub weekly: Vec<u64>,
    pub monthly: Vec<u64>,
}
/// 插件状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginStatus {
    Active,
    Deprecated,
    Yanked,
    Unpublished,
}
/// 搜索引擎
#[derive(Debug, Clone)]
pub struct SearchEngine {
    /// 搜索索引
    search_index: Arc<SearchIndex>,
    /// 排名算法
    ranking_algorithm: Arc<RankingAlgorithm>,
}
/// 搜索索引
#[derive(Debug, Clone)]
pub struct SearchIndex {
    /// 全文搜索索引
    fulltext_index: HashMap<String, HashSet<PluginId>>,
    /// 标签搜索索引
    tag_search_index: HashMap<String, HashSet<PluginId>>,
    /// 作者搜索索引
    author_search_index: HashMap<String, HashSet<PluginId>>,
}
/// 排名算法
#[derive(Debug, Clone)]
pub struct RankingAlgorithm {
    /// 权重配置
    weights: RankingWeights,
}
/// 排名权重
#[derive(Debug, Clone)]
pub struct RankingWeights {
    pub relevance: f64,
    pub downloads: f64,
    pub rating: f64,
    pub recency: f64,
    pub verified: f64,
}
/// 搜索查询
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub query: String,
    pub filters: SearchFilters,
    pub sort: SortOption,
    pub pagination: Pagination,
}
/// 搜索过滤器
#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
    pub rating_min: Option<f64>,
    pub verified_only: bool,
    pub free_only: bool,
    pub version_constraint: Option<VersionConstraint>,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}
impl std::hash::Hash for SearchFilters {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.categories.hash(state);
        self.tags.hash(state);
        self.authors.hash(state);
        if let Some(rating) = self.rating_min {
            // 将 f64 转换为可哈希的值
            rating.to_bits().hash(state);
        }
        self.verified_only.hash(state);
        self.free_only.hash(state);
        self.version_constraint.hash(state);
        if let Some((start, end)) = &self.date_range {
            start.timestamp().hash(state);
            end.timestamp().hash(state);
        }
    }
}
/// 排序选项
#[derive(Debug, Clone)]
pub enum SortOption {
    Relevance,
    Downloads,
    Rating,
    RecentlyUpdated,
    NewlyPublished,
    Name,
}
impl std::hash::Hash for SortOption {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // 使用变体的名称进行哈希
        match self {
            SortOption::Relevance => "Relevance".hash(state),
            SortOption::Downloads => "Downloads".hash(state),
            SortOption::Rating => "Rating".hash(state),
            SortOption::RecentlyUpdated => "RecentlyUpdated".hash(state),
            SortOption::NewlyPublished => "NewlyPublished".hash(state),
            SortOption::Name => "Name".hash(state),
        }
    }
}
/// 分页信息
#[derive(Debug, Clone)]
pub struct Pagination {
    pub page: usize,
    pub per_page: usize,
}
impl std::hash::Hash for Pagination {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.page.hash(state);
        self.per_page.hash(state);
    }
}
/// 搜索结果
#[derive(Debug, Clone)]
pub struct SearchResults {
    pub plugins: Vec<PluginSearchResult>,
    pub total: usize,
    pub took_ms: u64,
    pub facets: SearchFacets,
}
/// 插件搜索结果
#[derive(Debug, Clone)]
pub struct PluginSearchResult {
    pub plugin: PluginMetadata,
    pub score: f64,
    pub highlights: Vec<String>,
}
/// 搜索面
#[derive(Debug, Clone)]
pub struct SearchFacets {
    pub categories: Vec<FacetCount>,
    pub tags: Vec<FacetCount>,
    pub authors: Vec<FacetCount>,
}
/// 分面计数
#[derive(Debug, Clone)]
pub struct FacetCount {
    pub value: String,
    pub count: u64,
}
/// 评分系统
#[derive(Debug, Clone)]
pub struct RatingSystem {
    /// 评分存储
    pub ratings: Arc<RatingStorage>,
    /// 统计计算器
    stats_calculator: Arc<RatingStatsCalculator>,
}
/// 评分存储
#[derive(Debug, Clone)]
pub struct RatingStorage {
    /// 用户评分
    pub user_ratings: HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating, (PluginId, String), UserRating, std::collections::HashMap<(PluginId, String), UserRating, (PluginId, String), UserRating>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>,
    /// 评分聚合
    pub rating_aggregates: HashMap<PluginId, RatingAggregate>,
}
/// 用户评分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRating {
    pub user_id: String,
    pub rating: u8,
    pub review: Option<String>,
    pub created_at: DateTime<Utc>,
    pub helpful_votes: u64,
}
/// 评分聚合
#[derive(Debug, Clone)]
pub struct RatingAggregate {
    pub average: f64,
    pub count: u64,
    pub distribution: RatingDistribution,
    pub last_updated: DateTime<Utc>,
}
/// 评分统计计算器
#[derive(Debug, Clone)]
pub struct RatingStatsCalculator;
/// 审核系统
#[derive(Debug, Clone)]
pub struct ReviewSystem {
    /// 审核队列
    review_queue: Arc<ReviewQueue>,
    /// 自动审核器
    auto_reviewer: Arc<AutoReviewer>,
}
/// 审核队列
#[derive(Debug, Clone)]
pub struct ReviewQueue {
    /// 待审核插件
    pending_reviews: HashMap<PluginId, ReviewTicket>,
    /// 审核历史
    review_history: HashMap<PluginId, Vec<ReviewRecord>>,
}
/// 审核票据
#[derive(Debug, Clone)]
pub struct ReviewTicket {
    pub plugin_id: PluginId,
    pub submitted_at: DateTime<Utc>,
    pub reviewer_assigned: Option<String>,
    pub status: ReviewStatus,
    pub priority: ReviewPriority,
}
/// 审核状态
#[derive(Debug, Clone)]
pub enum ReviewStatus {
    Pending,
    InProgress,
    Approved,
    Rejected,
    RequiresChanges,
}
/// 审核优先级
#[derive(Debug, Clone)]
pub enum ReviewPriority {
    Low,
    Normal,
    High,
    Critical,
}
/// 审核记录
#[derive(Debug, Clone)]
pub struct ReviewRecord {
    pub reviewer_id: String,
    pub action: ReviewAction,
    pub comments: Option<String>,
    pub timestamp: DateTime<Utc>,
}
/// 审核动作
#[derive(Debug, Clone)]
pub enum ReviewAction {
    Submit,
    Approve,
    Reject,
    RequestChanges,
    Comment,
}
/// 自动审核器
#[derive(Debug, Clone)]
pub struct AutoReviewer {
    /// 安全扫描器
    security_scanner: Arc<SecurityScanner>,
    /// 质量分析器
    quality_analyzer: Arc<QualityAnalyzer>,
}
/// 安全扫描器
#[derive(Debug, Clone)]
pub struct SecurityScanner;
/// 质量分析器
#[derive(Debug, Clone)]
pub struct QualityAnalyzer;
/// 市场缓存
#[derive(Debug, Clone)]
pub struct MarketplaceCache {
    /// 插件详情缓存
    pub plugin_cache: HashMap<PluginId, PluginMetadata>,
    /// 搜索结果缓存
    pub search_cache: HashMap<String, SearchResults>,
    /// 缓存统计
    pub stats: CacheStats,
}
/// 缓存统计
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
}
impl PluginMarketplace {
    /// 创建新的插件市场实例
    pub fn new() -> Self {
        Self {
            plugin_index: Arc::new(Mutex::new(PluginIndex::new()))
            search_engine: Arc::new(Mutex::new(SearchEngine::new()))
            rating_system: Arc::new(Mutex::new(RatingSystem::new()))
            review_system: Arc::new(Mutex::new(ReviewSystem::new()))
            cache: Arc::new(Mutex::new(MarketplaceCache::new()))
        }
    }
    /// 初始化市场数据
    pub async fn initialize(&self) -> Result<()> {
        // 初始化插件索引
        self.plugin_index.initialize().await?;
        // 构建搜索索引
        self.search_engine.build_index().await?;
        // 预热缓存
        self.cache.warmup().await?;
        Ok(())
    }
    /// 搜索插件
    pub async fn search_plugins(&mut self, query: &SearchQuery) -> Result<SearchResults> {
        // 检查缓存
        let cache_key: _ = self.generate_cache_key(query);
        if let Some(cached_results) = self.cache.get_search_results(&cache_key).await {
            return Ok(cached_results);
        }
        // 执行搜索
        let start_time: _ = std::time::Instant::now();
        let results: _ = self.search_engine.search(query).await?;
        let took_ms: _ = start_time.elapsed().as_millis() as u64;
        // 计算分面
        let facets: _ = self.calculate_facets(&results).await?;
        let search_results: _ = SearchResults {
            plugins: results.clone(),
            total: results.len(),
            took_ms,
            facets,
        };
        // 缓存结果（克隆以避免移动）
        if let Some(cache) = Arc::get_mut(&mut self.cache) {
            cache.search_cache.insert(cache_key, search_results.clone());
        }
        Ok(search_results)
    }
    /// 获取插件详情
    pub async fn get_plugin_details(&mut self, plugin_id: &PluginId) -> Result<Option<PluginMetadata>> {
        // 检查缓存
        if let Some(cached) = self.cache.get_plugin(plugin_id).await {
            return Ok(Some(cached));
        }
        // 从索引获取
        let plugin: _ = self.plugin_index.get_plugin(plugin_id).await?;
        if let Some(ref plugin_data) = plugin {
            // 缓存插件数据
            if let Some(cache) = Arc::get_mut(&mut self.cache) {
                cache.plugin_cache.insert(plugin_id.clone(), plugin_data.clone());
            }
        }
        Ok(plugin)
    }
    /// 获取插件评分详情
    pub async fn get_plugin_rating(&self, plugin_id: &PluginId) -> Result<PluginRating> {
        self.rating_system.get_rating(plugin_id).await
    }
    /// 提交插件评分
    pub async fn rate_plugin(&mut self,
        plugin_id: &PluginId,
        user_id: &str,
        rating: u8,
        review: Option<String>,
    ) -> Result<()> {
        if let Some(rating_system) = Arc::get_mut(&mut self.rating_system) {
            rating_system.submit_rating(plugin_id, user_id, rating, review).await
        } else {
            Err(anyhow::anyhow!("Failed to get mutable reference to rating system"))
        }
    }
    /// 获取热门插件
    pub async fn get_featured_plugins(&self, limit: usize) -> Result<Vec<PluginMetadata> {
        self.plugin_index.get_featured_plugins(limit).await
    }
    /// 获取最新插件
    pub async fn get_recent_plugins(&self, limit: usize) -> Result<Vec<PluginMetadata> {
        self.plugin_index.get_recent_plugins(limit).await
    }
    /// 获取插件统计信息
    pub async fn get_plugin_stats(&self) -> Result<MarketplaceStats> {
        Ok(MarketplaceStats {
            total_plugins: self.plugin_index.total_plugins(),
            total_downloads: self.plugin_index.total_downloads(),
            average_rating: self.rating_system.get_average_rating().await,
            categories_count: self.plugin_index.categories_count(),
        })
    }
    /// 生成缓存键
    fn generate_cache_key(&self, query: &SearchQuery) -> String {

        let mut hasher = DefaultHasher::new();
        query.query.hash(&mut hasher);
        query.filters.hash(&mut hasher);
        query.sort.hash(&mut hasher);
        query.pagination.hash(&mut hasher);
        format!("search_{:x}", hasher.finish())
    }
    /// 计算搜索分面
    async fn calculate_facets(&self, plugins: &[PluginSearchResult]) -> Result<SearchFacets> {
        let mut category_counts = HashMap::new();
        let mut tag_counts = HashMap::new();
        let mut author_counts = HashMap::new();
        for result in plugins {
            // 统计分类
            for category in &result.plugin.categories {
                *category_counts.entry(category.clone()).or_insert(0) += 1;
            }
            // 统计标签
            for tag in &result.plugin.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
            // 统计作者
            let author_name: _ = &result.plugin.author.name;
            *author_counts.entry(author_name.clone()).or_insert(0) += 1;
        }
        let categories: _ = category_counts
            .into_iter()
            .map(|(value, count)| FacetCount { value, count })
            .collect();
        let tags: _ = tag_counts
            .into_iter()
            .map(|(value, count)| FacetCount { value, count })
            .collect();
        let authors: _ = author_counts
            .into_iter()
            .map(|(value, count)| FacetCount { value, count })
            .collect();
        Ok(SearchFacets {
            categories,
            tags,
            authors,
        })
    }
}
/// 市场统计信息
#[derive(Debug, Clone)]
pub struct MarketplaceStats {
    pub total_plugins: usize,
    pub total_downloads: u64,
    pub average_rating: f64,
    pub categories_count: usize,
}
// 实现各个组件的初始化方法
impl PluginIndex {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            tag_index: BTreeMap::new(),
            author_index: BTreeMap::new(),
            category_index: BTreeMap::new(),
        }
    }
    pub async fn initialize(&self) -> Result<()> {
        // 初始化索引数据
        Ok(())
    }
    pub async fn get_plugin(&self, plugin_id: &PluginId) -> Result<Option<PluginMetadata>> {
        Ok(self.plugins.get(plugin_id).cloned())
    }
    pub async fn get_featured_plugins(&self, limit: usize) -> Result<Vec<PluginMetadata> {
        let featured: Vec<_> = self.plugins
            .values()
            .filter(|p| p.featured)
            .cloned()
            .take(limit)
            .collect();
        Ok(featured)
    }
    pub async fn get_recent_plugins(&self, limit: usize) -> Result<Vec<PluginMetadata> {
        let recent: Vec<_> = self.plugins
            .values()
            .cloned()
            .take(limit)
            .collect();
        Ok(recent)
    }
    pub fn total_plugins(&self) -> usize {
        self.plugins.len()
    }
    pub fn total_downloads(&self) -> u64 {
        self.plugins.values().map(|p| p.downloads.total).sum()
    }
    pub fn categories_count(&self) -> usize {
        self.category_index.len()
    }
}
impl SearchEngine {
    pub fn new() -> Self {
        Self {
            search_index: Arc::new(Mutex::new(SearchIndex::new()))
            ranking_algorithm: Arc::new(Mutex::new(RankingAlgorithm::new()))
        }
    }
    pub async fn build_index(&self) -> Result<()> {
        // 构建搜索索引
        Ok(())
    }
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<PluginSearchResult> {
        let mut results = Vec::new();
        // 如果查询为空，返回所有插件
        if query.query.is_empty() {
            // 返回按排序的插件
            return Ok(results);
        }
        // 解析查询词
        let terms: _ = self.parse_query(&query.query);
        // 执行搜索
        for term in terms {
            let term_results: _ = self.search_by_term(&term, query).await?;
            results.extend(term_results);
        }
        // 去重
        results.sort_by(|a, b| a.plugin.plugin_id.name.cmp(&b.plugin.plugin_id.name));
        results.dedup_by(|a, b| a.plugin.plugin_id == b.plugin.plugin_id);
        // 应用过滤器
        results = self.apply_filters(results, &query.filters).await?;
        // 排序
        results = self.sort_results(results, &query.sort).await?;
        // 分页
        let start: _ = (query.pagination.page - 1) * query.pagination.per_page;
        let end: _ = start + query.pagination.per_page;
        let paginated_results: _ = if end <= results.len() {
            results[start..end].to_vec()
        } else {
            results[start..].to_vec()
        };
        Ok(paginated_results)
    }
    /// 解析查询字符串
    fn parse_query(&self, query: &str) -> Vec<String> {
        query.split_whitespace()
            .map(|s| s.to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    }
    /// 按词条搜索
    async fn search_by_term(&self, term: &str, query: &SearchQuery) -> Result<Vec<PluginSearchResult> {
        let mut results = Vec::new();
        // 模拟搜索结果（实际实现中会查询索引）
        // 这里应该根据 fulltext_index, tag_search_index 等进行搜索
        // 演示用：返回模拟结果
        if term.contains("test") {
            let plugin: _ = PluginMetadata {
                plugin_id: PluginId {
                    name: format!("test-plugin-{}", term),
                    version: Version::parse("1.0.0").unwrap(),
                },
                name: format!("Test Plugin {}", term),
                description: format!("A test plugin for {}", term),
                long_description: None,
                author: PluginAuthor {
                    id: "author1".to_string(),
                    name: "Test Author".to_string(),
                    email: None,
                    url: None,
                    avatar_url: None,
                },
                version: Version::parse("1.0.0").unwrap(),
                amberjs_version: VersionConstraint::parse("^1.0.0").unwrap(),
                license: "MIT".to_string(),
                homepage: None,
                repository: None,
                keywords: vec![term.to_string()],
                categories: vec!["testing".to_string()],
                tags: vec![term.to_string()],
                icon_url: None,
                screenshots: vec![],
                downloads: DownloadStats {
                    total: 1000,
                    recent: 100,
                    trend: DownloadTrend {
                        daily: vec![10; 30],
                        weekly: vec![70; 52],
                        monthly: vec![300; 12],
                    },
                },
                rating: PluginRating {
                    average: 4.5,
                    count: 50,
                    distribution: RatingDistribution {
                        five_star: 30,
                        four_star: 15,
                        three_star: 3,
                        two_star: 1,
                        one_star: 1,
                    },
                },
                published_at: Utc::now(),
                updated_at: Utc::now(),
                verified: true,
                featured: false,
                status: PluginStatus::Active,
            };
            results.push(PluginSearchResult {
                plugin,
                score: 0.95,
                highlights: vec![format!("Found in: {}", term)],
            });
        }
        Ok(results)
    }
    /// 应用搜索过滤器
    async fn apply_filters(
        &self,
        mut results: Vec<PluginSearchResult>,
        filters: &SearchFilters,
    ) -> Result<Vec<PluginSearchResult> {
        // 分类过滤
        if let Some(ref categories) = filters.categories {
            results.retain(|r| {
                r.plugin.categories.iter().any(|c| categories.contains(c))
            });
        }
        // 标签过滤
        if let Some(ref tags) = filters.tags {
            results.retain(|r| {
                r.plugin.tags.iter().any(|t| tags.contains(t))
            });
        }
        // 作者过滤
        if let Some(ref authors) = filters.authors {
            results.retain(|r| {
                authors.contains(&r.plugin.author.name)
            });
        }
        // 评分过滤
        if let Some(min_rating) = filters.rating_min {
            results.retain(|r| r.plugin.rating.average >= min_rating);
        }
        // 仅认证插件
        if filters.verified_only {
            results.retain(|r| r.plugin.verified);
        }
        // 免费插件
        if filters.free_only {
            results.retain(|r| r.plugin.license == "MIT" || r.plugin.license == "Apache-2.0");
        }
        // 版本约束过滤
        if let Some(ref constraint) = filters.version_constraint {
            results.retain(|r| constraint.matches(&r.plugin.version));
        }
        // 日期范围过滤
        if let Some((start, end)) = filters.date_range {
            results.retain(|r| {
                r.plugin.published_at >= start && r.plugin.published_at <= end
            });
        }
        Ok(results)
    }
    /// 排序搜索结果
    async fn sort_results(
        &self,
        mut results: Vec<PluginSearchResult>,
        sort: &SortOption,
    ) -> Result<Vec<PluginSearchResult> {
        match sort {
            SortOption::Relevance => {
                results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            }
            SortOption::Downloads => {
                results.sort_by(|a, b| b.plugin.downloads.total.cmp(&a.plugin.downloads.total));
            }
            SortOption::Rating => {
                results.sort_by(|a, b| {
                    b.plugin.rating.average.partial_cmp(&a.plugin.rating.average)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortOption::RecentlyUpdated => {
                results.sort_by(|a, b| b.plugin.updated_at.cmp(&a.plugin.updated_at));
            }
            SortOption::NewlyPublished => {
                results.sort_by(|a, b| b.plugin.published_at.cmp(&a.plugin.published_at));
            }
            SortOption::Name => {
                results.sort_by(|a, b| a.plugin.name.cmp(&b.plugin.name));
            }
        }
        Ok(results)
    }
}
impl SearchIndex {
    pub fn new() -> Self {
        Self {
            fulltext_index: HashMap::new(),
            tag_search_index: HashMap::new(),
            author_search_index: HashMap::new(),
        }
    }
}
impl RankingAlgorithm {
    pub fn new() -> Self {
        Self {
            weights: RankingWeights {
                relevance: 0.4,
                downloads: 0.2,
                rating: 0.2,
                recency: 0.1,
                verified: 0.1,
            },
        }
    }
}
impl RatingSystem {
    pub fn new() -> Self {
        Self {
            ratings: Arc::new(Mutex::new(RatingStorage::new()))
            stats_calculator: Arc::new(Mutex::new(RatingStatsCalculator)))
        }
    }
    pub async fn get_rating(&self, plugin_id: &PluginId) -> Result<PluginRating> {
        // 获取评分聚合数据
        if let Some(aggregate) = self.ratings.rating_aggregates.get(plugin_id) {
            Ok(PluginRating {
                average: aggregate.average,
                count: aggregate.count,
                distribution: aggregate.distribution.clone(),
            })
        } else {
            // 返回默认评分
            Ok(PluginRating {
                average: 0.0,
                count: 0,
                distribution: RatingDistribution {
                    five_star: 0,
                    four_star: 0,
                    three_star: 0,
                    two_star: 0,
                    one_star: 0,
                },
            })
        }
    }
    pub async fn submit_rating(
        &mut self,
        plugin_id: &PluginId,
        user_id: &str,
        rating: u8,
        review: Option<String>,
    ) -> Result<()> {
        // 验证评分范围
        if rating < 1 || rating > 5 {
            return Err(anyhow::anyhow!("Rating must be between 1 and 5"));
        }
        // 创建用户评分记录
        let user_rating: _ = UserRating {
            user_id: user_id.to_string(),
            rating,
            review,
            created_at: Utc::now(),
            helpful_votes: 0,
        };
        // 存储用户评分
        let rating_key: _ = (plugin_id.clone(), user_id.to_string());
        if let Some(ratings) = Arc::get_mut(&mut self.ratings) {
            ratings.user_ratings.insert(rating_key, user_rating);
        }
        // 重新计算聚合评分
        self.recalculate_aggregate(plugin_id).await?;
        Ok(())
    }
    pub async fn get_average_rating(&self) -> f64 {
        // 计算所有插件的平均评分
        let mut total_rating = 0.0;
        let mut count = 0;
        for aggregate in self.ratings.rating_aggregates.values() {
            total_rating += aggregate.average * aggregate.count as f64;
            count += aggregate.count;
        }
        if count > 0 {
            total_rating / count as f64
        } else {
            0.0
        }
    }
    /// 重新计算插件的聚合评分
    async fn recalculate_aggregate(&mut self, plugin_id: &PluginId) -> Result<()> {
        let plugin_ratings: Vec<_> = self.ratings
            .user_ratings
            .iter()
            .filter(|(key, _)| key.0 == *plugin_id)
            .map(|(_, rating)| rating)
            .collect();
        if plugin_ratings.is_empty() {
            return Ok(());
        }
        let count: _ = plugin_ratings.len() as u64;
        let total: u64 = plugin_ratings.iter().map(|r| r.rating as u64).sum();
        let average: _ = total as f64 / count as f64;
        // 计算评分分布
        let mut distribution = RatingDistribution {
            five_star: 0,
            four_star: 0,
            three_star: 0,
            two_star: 0,
            one_star: 0,
        };
        for rating in plugin_ratings {
            match rating.rating {
                5 => distribution.five_star += 1,
                4 => distribution.four_star += 1,
                3 => distribution.three_star += 1,
                2 => distribution.two_star += 1,
                1 => distribution.one_star += 1,
                _ => {}
            }
        }
        // 更新聚合数据
        let aggregate: _ = RatingAggregate {
            average,
            count,
            distribution,
            last_updated: Utc::now(),
        };
        if let Some(ratings) = Arc::get_mut(&mut self.ratings) {
            ratings.rating_aggregates.insert(plugin_id.clone(), aggregate);
        }
        Ok(())
    }
    /// 获取用户对插件的评分
    pub async fn get_user_rating(
        &self,
        plugin_id: &PluginId,
        user_id: &str,
    ) -> Result<Option<UserRating>> {
        let rating_key: _ = (plugin_id.clone(), user_id.to_string());
        Ok(self.ratings.user_ratings.get(&rating_key).cloned())
    }
    /// 投票帮助性
    pub async fn vote_helpful(&mut self,
        plugin_id: &PluginId,
        user_id: &str,
        helpful: bool,
    ) -> Result<()> {
        let rating_key: _ = (plugin_id.clone(), user_id.to_string());
        if let Some(ratings) = Arc::get_mut(&mut self.ratings) {
            if let Some(rating) = ratings.user_ratings.get_mut(&rating_key) {
                if helpful {
                    rating.helpful_votes += 1;
                } else {
                    rating.helpful_votes = rating.helpful_votes.saturating_sub(1);
                }
            }
        }
        Ok(())
    }
    /// 获取插件的所有评论
    pub async fn get_plugin_reviews(
        &self,
        plugin_id: &PluginId,
        limit: usize,
    ) -> Result<Vec<UserRating> {
        let mut reviews: Vec<_> = self.ratings
            .user_ratings
            .iter()
            .filter_map(|(key, rating)| {
                if key.0 == *plugin_id && rating.review.is_some() {
                    Some(rating.clone())
                } else {
                    None
                }
            })
            .collect();
        // 按帮助性投票数排序
        reviews.sort_by(|a, b| b.helpful_votes.cmp(&a.helpful_votes));
        // 限制数量
        if reviews.len() > limit {
            reviews.truncate(limit);
        }
        Ok(reviews)
    }
    /// 获取评分统计
    pub async fn get_rating_statistics(&self, plugin_id: &PluginId) -> Result<RatingStatistics> {
        let plugin_ratings: Vec<_> = self.ratings
            .user_ratings
            .iter()
            .filter(|(key, _)| key.0 == *plugin_id)
            .map(|(_, rating)| rating)
            .collect();
        if plugin_ratings.is_empty() {
            return Ok(RatingStatistics {
                total_reviews: 0,
                reviews_with_comments: 0,
                helpful_votes_total: 0,
                average_review_length: 0.0,
                rating_trend: RatingTrend::Stable,
            });
        }
        let total_reviews: _ = plugin_ratings.len() as u64;
        let reviews_with_comments: _ = plugin_ratings
            .iter()
            .filter(|r| r.review.is_some())
            .count() as u64;
        let helpful_votes_total: u64 = plugin_ratings.iter().map(|r| r.helpful_votes).sum();
        let total_length: usize = plugin_ratings
            .iter()
            .filter_map(|r| r.review.as_ref().map(|s| s.len())
            .sum();
        let average_review_length: _ = if reviews_with_comments > 0 {
            total_length as f64 / reviews_with_comments as f64
        } else {
            0.0
        };
        // 简化的趋势计算（实际中会更复杂）
        let rating_trend: _ = RatingTrend::Stable;
        Ok(RatingStatistics {
            total_reviews,
            reviews_with_comments,
            helpful_votes_total,
            average_review_length,
            rating_trend,
        })
    }
}
/// 评分统计信息
#[derive(Debug, Clone)]
pub struct RatingStatistics {
    pub total_reviews: u64,
    pub reviews_with_comments: u64,
    pub helpful_votes_total: u64,
    pub average_review_length: f64,
    pub rating_trend: RatingTrend,
}
/// 评分趋势
#[derive(Debug, Clone)]
pub enum RatingTrend {
    Rising,
    Falling,
    Stable,
}
impl RatingStorage {
    pub fn new() -> Self {
        Self {
            user_ratings: HashMap::new(),
            rating_aggregates: HashMap::new(),
        }
    }
}
impl RatingStatsCalculator {
    pub fn new() -> Self {
        Self
    }
}
impl ReviewSystem {
    pub fn new() -> Self {
        Self {
            review_queue: Arc::new(Mutex::new(ReviewQueue::new()))
            auto_reviewer: Arc::new(Mutex::new(AutoReviewer::new()))
        }
    }
}
impl ReviewQueue {
    pub fn new() -> Self {
        Self {
            pending_reviews: HashMap::new(),
            review_history: HashMap::new(),
        }
    }
}
impl AutoReviewer {
    pub fn new() -> Self {
        Self {
            security_scanner: Arc::new(Mutex::new(SecurityScanner)))
            quality_analyzer: Arc::new(Mutex::new(QualityAnalyzer)))
        }
    }
}
impl SecurityScanner {
    pub fn new() -> Self {
        Self
    }
}
impl QualityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl MarketplaceCache {
    pub fn new() -> Self {
        Self {
            plugin_cache: HashMap::new(),
            search_cache: HashMap::new(),
            stats: CacheStats {
                hits: 0,
                misses: 0,
                size: 0,
            },
        }
    }
    pub async fn warmup(&self) -> Result<()> {
        // 预热缓存
        Ok(())
    }
    pub async fn get_plugin(&self, plugin_id: &PluginId) -> Option<PluginMetadata> {
        self.plugin_cache.get(plugin_id).cloned()
    }
    pub async fn store_plugin(&mut self, plugin_id: &PluginId, plugin: PluginMetadata) {
        self.plugin_cache.insert(plugin_id.clone(), plugin);
    }
    pub async fn get_search_results(&self, cache_key: &str) -> Option<SearchResults> {
        self.search_cache.get(cache_key).cloned()
    }
    pub async fn store_search_results(&mut self, cache_key: String, results: SearchResults) {
        self.search_cache.insert(cache_key, results);
    }
}