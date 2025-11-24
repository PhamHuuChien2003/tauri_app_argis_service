export interface ExampleResult {
  status: string;
  address: string;
  province: string;
  district: string;
  ward: string;

  poi_vn?: string | null;
  poi_en?: string | null;
  poi_ex?: string | null;

  type?: string | null;
  sub_type?: string | null;
  poi_st_sd?: string | null;

  room?: string | null;
  house_num?: string | null;
  buaname?: string | null;
  st_name?: string | null;
  sub_com?: string | null;

  phone?: string | null;
  fax?: string | null;
  web?: string | null;
  mail?: string | null;

  brandname?: string | null;
  import?: string | null;
  status_detail?: string | null;
  note?: string | null;
  dine?: string | null;
  update_?: string | null;
  source?: string | null;
  gen_type?: string | null;
  perform?: string | null;
  dup?: string | null;
  explain?: string | null;
  classify?: string | null;
  dtrend?: string | null;

  google_id?: string | null;
  be_id?: string | null;
}

export interface IncomingData {
  lat: number;
  lng: number;
}