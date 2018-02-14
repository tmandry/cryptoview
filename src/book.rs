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

impl<'a> From<&'a NewOrderEvent<'a>> for Order {
    fn from(o: &NewOrderEvent<'a>) -> Self {
        Order{
            id: o.order_id.to_owned(),
            side: o.side,
            px: o.px,
            orig_size: o.orig_size,
            open_size: o.open_size,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Time(u64);
#[derive(Copy, Clone, Debug)]
pub struct Sequence(u64);

// Event structs. These include the data that is common to all feeds.

pub struct NewOrderEvent<'a> {
    time: Time,
    seq: Sequence,
    order_id: &'a str,
    side: Side,
    px: Price,
    orig_size: Price,
    open_size: Price,
}

pub struct OpenEvent<'a> {
    time: Time,
    seq: Sequence,
    order_id: &'a str,
    remaining_size: Price,
}
/*
pub struct MatchEvent<'a> {
    time: Time,
    seq: Sequence,
    maker_order_id: &'a str,
    taker_order_id: &'a str,
    side: Side,
    price: Price,
    size: Price,
}

pub struct ChangeEvent<'a> {
    time: Time,
    seq: Sequence,
    order_id: &'a str,
    price: Option<Price>,  // None indicates market order
    old_size_or_funds: Price,
    new_size_or_funds: Price,
}

pub struct DoneEvent<'a> {
    time: Time,
    seq: Sequence,
    order_id: &'a str,
    reason: DoneReason,
}

pub enum DoneReason {
    Filled,
    Canceled,
}
*/
pub trait Level3FeedListener {
    fn on_add<'a>(&mut self, order: &NewOrderEvent<'a>);
    fn on_open<'a>(&mut self, event: &OpenEvent<'a>);
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
    fn on_add<'a>(&mut self, event: &NewOrderEvent<'a>) {
        let ord = Rc::new(RefCell::new(Order::from(event)));
        let entry = {
            self.orders.insert(event.order_id.to_owned(), ord.clone());
            match &event.side {
                &Side::Bid => self.bid.entry(event.px),
                &Side::Ask => self.ask.entry(event.px),
            }
        };
        entry.or_insert(PriceLevel::new()).add(ord);
    }

    fn on_open<'a>(&mut self, event: &OpenEvent<'a>) {
        let (side, px) = {
            let mut order = self.orders.get(event.order_id).expect("Unknown order ID").borrow_mut();
            order.open_size = event.remaining_size;
            (order.side, order.px)
        };

        let level = self.price_level_mut(side, px)
                        .expect("Price level with order doesn't exist!");
        level.open_size += event.remaining_size;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(p: f64) -> Price { Price::from(p) }

    fn new_event(order_id: &str, side: Side, px: Price, orig_size: Price) -> NewOrderEvent {
        NewOrderEvent{ seq: Sequence(0), time: Time(0), order_id, side, px, orig_size, open_size: orig_size }
    }

    fn open_event(order_id: &str, remaining_size: Price) -> OpenEvent {
        OpenEvent{ seq: Sequence(0), time: Time(0), order_id, remaining_size }
    }

    #[test]
    fn total_size() {
        let mut book = Book::new();
        book.on_add(&new_event(&"order1", Side::Bid, px(10.00), px(100.)));
        assert_eq!(px(100.), book.price_level(Side::Bid, px(10.00)).unwrap().total_size());
        book.on_add(&new_event(&"order2", Side::Bid, px(10.00), px(90.)));
        book.on_add(&new_event(&"order3", Side::Ask, px(10.01), px(90.)));
        assert_eq!(px(190.), book.price_level(Side::Bid, px(10.00)).unwrap().total_size());
        assert_eq!(px(90.), book.price_level(Side::Ask, px(10.01)).unwrap().total_size());
    }

    #[test]
    fn open_size() {
        let mut book = Book::new();
        book.on_add(&new_event(&"order1", Side::Bid, px(10.00), px(100.)));
        assert_eq!(Price::zero(), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
        book.on_add(&new_event(&"order2", Side::Bid, px(10.00), px(90.)));
        assert_eq!(Price::zero(), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
        book.on_open(&open_event(&"order2", px(90.)));
        assert_eq!(px(90.), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
    }
}
