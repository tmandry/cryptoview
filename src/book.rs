/// Implementation of a level 3 book.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;
use price::Price;

#[derive(Copy, Clone, Debug)]
pub enum Side {
    Bid,
    Ask
}

pub struct Book {
    bid: OrdersByPrice,
    ask: OrdersByPrice,
    orders: HashMap<String, Rc<Order>>
}

pub struct PriceLevel {
    orders: VecDeque<Rc<Order>>,
    total_size: Price,
}

impl PriceLevel {
    fn new() -> PriceLevel {
        PriceLevel{ orders: VecDeque::new(), total_size: Price::zero() }
    }

    fn add(&mut self, ord: Rc<Order>) {
        self.total_size += ord.size;
        self.orders.push_back(ord);
    }

    pub fn total_size(&self) -> Price { self.total_size }
}

type OrdersByPrice = BTreeMap<Price, PriceLevel>;

pub struct Order {
    id: String,
    side: Side,
    px: Price,
    size: Price,
}

impl Order {
    pub fn new(id: String, side: Side, px: Price, size: Price) -> Order {
        Order{id, side, px, size}
    }
}

pub trait OrderInfo {
    fn id(&self) -> &str;
    fn side(&self) -> Side;
    fn price(&self) -> Price;
    fn size(&self) -> Price;
}

impl OrderInfo for Order {
    fn id(&self) -> &str { &self.id }
    fn side(&self) -> Side { self.side }
    fn price(&self) -> Price { self.px }
    fn size(&self) -> Price { self.size }
}

impl Order {
    // From<OrderInfo> trait doesn't work, because Order: OrderInfo and this conflicts with the
    // default reflexive implementation.

    fn from_order_info<O: OrderInfo>(o: &O) -> Self {
        Order{ id: o.id().to_string(), side: o.side(), px: o.price(), size: o.size() }
    }
}

pub struct Time(u64);
pub struct Sequence(u64);

pub trait OrderEvent {
    fn time(&self) -> Time;
    fn seq(&self) -> Sequence;
}

pub trait OpenEvent : OrderEvent {
    fn order_id(&self) -> &str;
    fn remaining_size(&self) -> Price;
}

pub trait MatchEvent : OrderEvent {
    fn maker_order_id(&self) -> &str;
    fn taker_order_id(&self) -> &str;
    fn side(&self) -> Side;
    fn price(&self) -> Price;
    fn size(&self) -> Price;
}

pub trait ChangeEvent : OrderEvent {
    fn order_id(&self) -> &str;
    fn price(&self) -> Option<Price>;  // None indicates market order
    fn old_size_or_funds(&self) -> Price;
    fn new_size_or_funds(&self) -> Price;
}

pub enum DoneReason {
    Filled,
    Canceled,
}

pub trait DoneEvent : OrderEvent {
    fn order_id(&self) -> &str;
    //fn remaining_size(&self) -> Price;
    fn reason(&self) -> &DoneReason;
}

pub trait FeedListener {
    fn on_add<O: OrderInfo>(&mut self, order: &O);
    //fn on_open<E: OpenEvent>(&mut self, event: &E);
    //fn on_match<E: MatchEvent>(&mut self, event: &E);
    //fn on_change<E: ChangeEvent>(&mut self, event: &E);
    //fn on_done<E: DoneEvent>(&mut self, event: &E);
}

impl Book {
    pub fn new() -> Book {
        Book{ bid: OrdersByPrice::new(), ask: OrdersByPrice::new(), orders: HashMap::new() }
    }

    pub fn price_level(&self, side: Side, px: Price) -> Option<&PriceLevel> {
        match side {
            Side::Bid => self.bid.get(&px),
            Side::Ask => self.ask.get(&px),
        }
    }
}

impl FeedListener for Book {
    fn on_add<O: OrderInfo>(&mut self, order: &O) {
        let order = Rc::new(Order::from_order_info(order));
        self.orders.insert(order.id.clone(), order.clone());
        let entry = match &order.side {
            &Side::Bid => self.bid.entry(order.px),
            &Side::Ask => self.ask.entry(order.px),
        };
        entry.or_insert(PriceLevel::new()).add(order);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(p: f64) -> Price { Price::from(p) }

    #[test]
    fn total_qty() {
        let mut book = Book::new();
        book.on_add(&Order::new("order1".to_string(), Side::Bid, px(10.00), px(100.)));
        book.on_add(&Order::new("order2".to_string(), Side::Bid, px(10.00), px(90.)));
        assert_eq!(px(190.), book.price_level(Side::Bid, px(10.00)).unwrap().total_size());
    }
}
