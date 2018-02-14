/// Implementation of a level 3 book.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::cell::RefCell;
use std::rc::Rc;
use price::Price;

#[derive(Copy, Clone, Debug)]
pub enum Side {
    Bid,
    Ask
}

pub struct PriceLevel {
    orders: VecDeque<Rc<RefCell<Order>>>,
    total_size: Price,
    open_size: Price,
}

impl PriceLevel {
    fn new() -> PriceLevel {
        PriceLevel{
            orders: VecDeque::new(),
            total_size: Price::zero(),
            open_size: Price::zero()
        }
    }

    fn add(&mut self, ord: Rc<RefCell<Order>>) {
        self.total_size += ord.borrow().orig_size;
        self.orders.push_back(ord);
    }

    pub fn total_size(&self) -> Price { self.total_size }
    pub fn open_size(&self) -> Price { self.open_size }
}

pub trait Level2EventListener {
    //fn on_new_level(side: Side, price: Price) -> LevelState;
    //fn on_remove_level(side: Side, price: Price, level_state: LevelState);
    fn on_level_change(side: Side, price: Price, old_size: Price, new_size: Price, /*level_state: &mut LevelState*/);
}

type OrdersByPrice = BTreeMap<Price, PriceLevel>;

pub struct Order {
    id: String,
    side: Side,
    px: Price,
    orig_size: Price,
    open_size: Price,
}

impl Order {
    pub fn new(id: String, side: Side, px: Price, size: Price) -> Order {
        Order{id, side, px, orig_size: size, open_size: size}
    }
}

pub trait OrderInfo {
    fn id(&self) -> &str;
    fn side(&self) -> Side;
    fn price(&self) -> Price;
    fn orig_size(&self) -> Price;
    fn open_size(&self) -> Price;
}

impl OrderInfo for Order {
    fn id(&self) -> &str { &self.id }
    fn side(&self) -> Side { self.side }
    fn price(&self) -> Price { self.px }
    fn orig_size(&self) -> Price { self.orig_size }
    fn open_size(&self) -> Price { self.orig_size }
}

impl Order {
    // From<OrderInfo> trait doesn't work, because Order: OrderInfo and this conflicts with the
    // default reflexive implementation.

    fn from_order_info<O: OrderInfo>(o: &O) -> Self {
        Order{
            id: o.id().to_string(),
            side: o.side(),
            px: o.price(),
            orig_size: o.orig_size(),
            open_size: o.open_size()
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Time(u64);
#[derive(Copy, Clone, Debug)]
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

pub trait Level3FeedListener {
    fn on_add<O: OrderInfo>(&mut self, order: &O);
    fn on_open<E: OpenEvent>(&mut self, event: &E);
    //fn on_match<E: MatchEvent>(&mut self, event: &E);
    //fn on_change<E: ChangeEvent>(&mut self, event: &E);
    //fn on_done<E: DoneEvent>(&mut self, event: &E);
}

pub struct Book {
    bid: OrdersByPrice,
    ask: OrdersByPrice,
    orders: HashMap<String, Rc<RefCell<Order>>>
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

    fn price_level_mut(&mut self, side: Side, px: Price) -> Option<&mut PriceLevel> {
        match side {
            Side::Bid => self.bid.get_mut(&px),
            Side::Ask => self.ask.get_mut(&px),
        }
    }
}

impl Level3FeedListener for Book {
    fn on_add<O: OrderInfo>(&mut self, order: &O) {
        let ord = Rc::new(RefCell::new(Order::from_order_info(order)));
        let entry = {
            let order = ord.borrow();
            self.orders.insert(order.id.clone(), ord.clone());
            match &order.side {
                &Side::Bid => self.bid.entry(order.px),
                &Side::Ask => self.ask.entry(order.px),
            }
        };
        entry.or_insert(PriceLevel::new()).add(ord);
    }

    fn on_open<E: OpenEvent>(&mut self, event: &E) {
        let (side, px) = {
            let mut order = self.orders.get(event.order_id()).expect("Unknown order ID").borrow_mut();
            order.open_size = event.remaining_size();
            (order.side, order.px)
        };

        let level = self.price_level_mut(side, px)
                        .expect("Price level with order doesn't exist!");
        level.open_size += event.remaining_size();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(p: f64) -> Price { Price::from(p) }

    #[test]
    fn total_size() {
        let mut book = Book::new();
        book.on_add(&Order::new("order1".to_string(), Side::Bid, px(10.00), px(100.)));
        assert_eq!(px(100.), book.price_level(Side::Bid, px(10.00)).unwrap().total_size());
        book.on_add(&Order::new("order2".to_string(), Side::Bid, px(10.00), px(90.)));
        book.on_add(&Order::new("order3".to_string(), Side::Ask, px(10.01), px(90.)));
        assert_eq!(px(190.), book.price_level(Side::Bid, px(10.00)).unwrap().total_size());
        assert_eq!(px(90.), book.price_level(Side::Ask, px(10.01)).unwrap().total_size());
    }

    struct MyOpenEvent {
        time: Time,
        seq: Sequence,
        order_id: String,
        remaining_size: Price,
    }

    impl MyOpenEvent {
        fn new(order_id: &str, remaining_size: Price) -> MyOpenEvent {
            MyOpenEvent{ time: Time(0), seq: Sequence(0), order_id: order_id.to_owned(), remaining_size }
        }
    }
    impl OrderEvent for MyOpenEvent {
        fn time(&self) -> Time { self.time }
        fn seq(&self) -> Sequence { self.seq }
    }
    impl OpenEvent for MyOpenEvent {
        fn order_id(&self) -> &str { &self.order_id }
        fn remaining_size(&self) -> Price { self.remaining_size }
    }

    #[test]
    fn open_size() {
        let mut book = Book::new();
        book.on_add(&Order::new("order1".to_string(), Side::Bid, px(10.00), px(100.)));
        assert_eq!(Price::zero(), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
        book.on_add(&Order::new("order2".to_string(), Side::Bid, px(10.00), px(90.)));
        assert_eq!(Price::zero(), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
        book.on_open(&MyOpenEvent::new("order2", px(90.)));
        assert_eq!(px(90.), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
    }
}
