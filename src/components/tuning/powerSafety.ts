export interface PowerSafetySnapshot {
  sample_count: number;
  span_ms: number;
  sample_age_ms: number | null;
  load_pct: number | null;
  power_p95_w: number | null;
  power_source: 'measured' | 'estimated' | 'unavailable';
  temperature_c: number | null;
  temperature_age_ms: number | null;
  temperature_slope_c_s: number | null;
  thermal_gain_c_w: number | null;
  ready: boolean;
}

export interface PowerAssessment {
  level: 'low' | 'moderate' | 'high';
  requires_confirmation: boolean;
  reasons: string[];
  predicted_temperature_c: number | null;
  snapshot?: PowerSafetySnapshot;
}
