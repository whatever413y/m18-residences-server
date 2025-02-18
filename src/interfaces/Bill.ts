export default interface Bill {
    tenant_id: number;
    room_charges: number;
    electric_charges: number;
    additional_charges: number;
    additional_description: string;
    total_amount: number;
}