export default interface ElectricityReading {
    tenant_id: number;
    prev_reading: number;
    curr_reading: number;
    consumption: number;
}