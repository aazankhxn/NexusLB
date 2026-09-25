use crate::adaptive::AdaptiveScheduler;
use crate::consistent_hash::ConsistentHashScheduler;
use crate::ewma_latency::EwmaLatencyScheduler;
use crate::ip_hash::IpHashScheduler;
use crate::least_connections::LeastConnectionsScheduler;
use crate::least_latency::LeastLatencyScheduler;
use crate::power_of_two::PowerOfTwoChoicesScheduler;
use crate::random::RandomScheduler;
use crate::round_robin::RoundRobinScheduler;
use crate::traits::Scheduler;
use crate::weighted_round_robin::WeightedRoundRobinScheduler;
use nexuslb_core::types::AlgorithmType;
use std::sync::Arc;

pub fn create_scheduler(algorithm: AlgorithmType) -> Arc<dyn Scheduler> {
    match algorithm {
        AlgorithmType::RoundRobin => Arc::new(RoundRobinScheduler::new()),
        AlgorithmType::WeightedRoundRobin => Arc::new(WeightedRoundRobinScheduler::new()),
        AlgorithmType::LeastConnections => Arc::new(LeastConnectionsScheduler::new()),
        AlgorithmType::Random => Arc::new(RandomScheduler::new()),
        AlgorithmType::IpHash => Arc::new(IpHashScheduler::new()),
        AlgorithmType::ConsistentHash => Arc::new(ConsistentHashScheduler::new()),
        AlgorithmType::PowerOfTwoChoices => Arc::new(PowerOfTwoChoicesScheduler::new()),
        AlgorithmType::LeastLatency => Arc::new(LeastLatencyScheduler::new()),
        AlgorithmType::EwmaLatency => Arc::new(EwmaLatencyScheduler::new()),
        AlgorithmType::Adaptive => Arc::new(AdaptiveScheduler::new()),
    }
}
